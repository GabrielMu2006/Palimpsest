#!/usr/bin/env ruby
# Collect evidence only from prebuilt, frozen release executables. No retries.
require 'json'
require 'digest'
require 'open3'
require 'time'

ROOT = File.expand_path('..', __dir__)
Dir.chdir(ROOT)
DATA = 'docs/reports/data'
PREFIX = "#{DATA}/kfix-v2"

def sources
  paths = IO.popen(['git', 'ls-files', '-co', '--exclude-standard', '-z'], &:read).split("\0").uniq
  paths.select { |p| File.file?(p) && (p.end_with?('.rs') || %w[MASTER_SPEC.md Cargo.toml Cargo.lock rust-toolchain.toml tools/collect-kfix-v2.rb].include?(p) || p.end_with?('/Cargo.toml')) }
       .sort.to_h { |p| [p, Digest::SHA256.file(p).hexdigest] }
end

def capture(command)
  output, status = Open3.capture2e(*command)
  raise "failed #{command.inspect}: #{output}" unless status.success?
  output.strip
end

def environment
  path = "#{PREFIX}-environment.json"
  raise "refusing to overwrite #{path}" if File.exist?(path)
  identity = sources
  env = {
    'recorded_at' => Time.now.iso8601, 'branch' => capture(%w[git branch --show-current]),
    'head' => capture(%w[git rev-parse HEAD]), 'dirty_status' => capture(%w[git status --short]),
    'cpu' => capture(%w[sysctl -n machdep.cpu.brand_string]),
    'memory_bytes' => Integer(capture(%w[sysctl -n hw.memsize])),
    'cpus' => Integer(capture(%w[sysctl -n hw.ncpu])),
    'os' => capture(%w[sw_vers]), 'rustc' => capture(%w[rustc --version]),
    'cargo' => capture(%w[cargo --version]), 'sources_sha256' => identity,
    'binaries_sha256' => %w[target/release/examples/action_execution_bench target/release/examples/kernel_bench target/release/examples/render_snapshot_bench target/release/palimpsest-bench-memory].to_h { |p| [p, Digest::SHA256.file(p).hexdigest] },
    'protocol' => 'release; no concurrent builds/tests; timings two complete warmups and ten samples; RSS three fresh processes/case; no sample retries'
  }
  raise 'reference machine mismatch' unless env['cpu'].include?('M5') && env['memory_bytes'] == 17_179_869_184
  File.open(path, File::WRONLY | File::CREAT | File::EXCL) { |f| f.puts JSON.pretty_generate(env) }
  puts "frozen #{identity.length} source files"
end

def verify_sources
  expected = JSON.parse(File.read("#{PREFIX}-environment.json"))
  raise 'source identity changed; do not mix measurements' unless sources == expected.fetch('sources_sha256')
  expected.fetch('binaries_sha256').each { |p, hash| raise "binary changed: #{p}" unless Digest::SHA256.file(p).hexdigest == hash }
end

def measure(path, commands)
  verify_sources
  File.open(path, File::WRONLY | File::CREAT | File::EXCL) do |file|
    commands.each do |command|
      started = Time.now.iso8601
      puts "START #{started} #{command.join(' ')}"
      $stdout.flush
      stdout = +''
      stderr = +''
      status = nil
      Open3.popen3(*command) do |stdin, out, err, wait|
        stdin.close
        reader = Thread.new { stdout << out.read }
        err.each_line { |line| stderr << line; puts line; $stdout.flush }
        reader.join
        status = wait.value
      end
      record = { command: command, started_at: started, ended_at: Time.now.iso8601, exit_code: status.exitstatus, stderr: stderr }
      File.open("#{PREFIX}-commands.jsonl", 'a') { |log| log.puts JSON.generate(record) }
      # Preserve the exact output even on failure; no silent retries/filtering.
      file.write(stdout)
      file.write("\n") unless stdout.empty? || stdout.end_with?("\n")
      file.flush
      raise "measurement failed: #{command.inspect}" unless status.success?
      JSON.parse(stdout)
      verify_sources
      puts "DONE #{Time.now.iso8601} #{File.basename(command[0])}"
    end
  end
end

def validate
  verify_sources
  counts = {}
  close = ->(actual, expected) { raise "rate/precision mismatch #{actual} != #{expected}" unless (actual - expected).abs <= [1e-9, expected.abs * 1e-12].max }
  %w[action kernel render].each do |kind|
    rows = File.readlines("#{PREFIX}-#{kind}-timing.jsonl").map { |line| JSON.parse(line) }
    raise 'missing timing command' unless rows.length == (kind == 'render' ? 1 : 2)
    rows.each do |row|
      series = row.fetch('samples_series')
      smoke = kind == 'kernel' && row['seconds'] == 86_400
      n = smoke ? 1 : 10
      raise 'sample/warmup count' unless row['samples'] == n && series.length == n && row['warmups'] == (smoke ? 0 : 2)
      raise 'sample indices' unless series.map { |s| s['index'] } == (0...n).to_a
      stable = case kind
               when 'action' then %w[transitions checksum stats events_total events_digest queue_depth stale_nodes]
               when 'kernel' then %w[rounds transitions decisions events_total events_digest checksum queue_max_observed]
               else %w[checksum total_bytes terrain_bytes sites_bytes persons_bytes metrics_bytes envelope_bytes per_person_bytes]
               end
      stable.each { |key| raise "nondeterministic #{key}" unless series.map { |s| s.fetch(key) }.uniq.length == 1 }
      if kind == 'render'
        %w[build serialize].each do |field|
          sorted = series.map { |s| s.fetch("#{field}_ns") }.sort
          close.call(row["min_#{field}_us"], sorted.first / 1000.0)
          close.call(row["median_#{field}_us"], sorted[n / 2] / 1000.0)
          close.call(row["max_#{field}_us"], sorted.last / 1000.0)
          series.each { |s| close.call(s["#{field}_us"], s["#{field}_ns"] / 1000.0) }
        end
        series.each do |s|
          raise 'section size mismatch' unless s['total_bytes'] == %w[terrain sites persons metrics envelope].sum { |key| s["#{key}_bytes"] }
          close.call(s['per_person_bytes'], s['persons_bytes'].to_f / row['persons'])
        end
        raise 'render fixture mismatch' unless row['schema_version'] == 2 && row['seconds'] == 600 && row['persons'] == 100
      else
        walls = series.map { |s| s['wall_ns'] }.sort
        median_key = kind == 'action' ? 'median_ns' : 'median_wall_ns'
        raise 'upper median mismatch' unless row[median_key] == walls[n / 2]
        series.each do |s|
          close.call(s['wall_seconds'], s['wall_ns'] / 1e9)
          counters = kind == 'kernel' ? %w[rounds transitions decisions events] : %w[transitions]
          counters.each { |key| close.call(s["#{key}_per_wall_second"], s[key == 'events' ? 'events_total' : key].to_f / s['wall_seconds']) }
        end
        if kind == 'action'
          raise 'action min/max or fixture' unless row['min_ns'] == walls.first && row['max_ns'] == walls.last && row['seed'] == 25_025 && row['seconds'] == 172_800
        else
          raise 'kernel fixture' unless row['seed'] == 42 && row['persons'] == 100 && row['spawn_layout'] == 'colocated_first_walkable' && row['work_budget'] == 1024
          close.call(row['min_wall_seconds'], walls.first / 1e9)
          close.call(row['max_wall_seconds'], walls.last / 1e9)
          close.call(row['median_sim_per_wall'], row['seconds'] / (walls[n / 2] / 1e9))
        end
      end
    end
    counts[kind] = rows.sum { |row| row['samples'] }
  end
  rows = File.readlines("#{PREFIX}-memory.jsonl").map { |line| JSON.parse(line) }
  raise 'memory case list' unless rows.map { |r| r['case'] } == %w[action-100 action-1000 kernel-100-year render-control-100 render-snapshot-100]
  pids = []
  ambiguous = 0
  rows.each do |row|
    samples = row['samples']
    raise 'cold protocol' unless samples.length == 3 && row['memory_warmups'] == 0 && row['sampling'] == 'fresh_process_per_sample'
    raise 'cold truth mismatch' unless samples.map { |s| s['checksum'] }.uniq.length == 1
    samples.each_with_index do |s, i|
      raise 'memory identity' unless s['sample_index'] == i && s['case'] == row['case'] && s['method'] == 'macos_kernel_rss_high_water_v1'
      pids << s['pid']
      %w[cold operation].each do |interval|
        v = s[interval]
        if v['proof'] == 'ambiguous_prior_peak'
          raise 'unproven cold / numeric ambiguous operation' unless interval == 'operation' && v['peak_increment_bytes'].nil?
          ambiguous += 1
        else
          b, e = v['baseline'], v['end']
          expected_proof = b['current_bytes'] == b['lifetime_peak_bytes'] ? 'baseline_at_lifetime_peak' : 'new_lifetime_peak_in_interval'
          raise 'invalid peak proof' unless v['proof'] == expected_proof && (b['current_bytes'] == b['lifetime_peak_bytes'] || e['lifetime_peak_bytes'] > b['lifetime_peak_bytes'])
          raise 'RSS units/increment' unless v['peak_increment_bytes'] == e['lifetime_peak_bytes'] - b['current_bytes']
        end
      end
    end
    sorted = samples.map { |s| s['cold']['peak_increment_bytes'] }.sort
    %w[min median max].zip(sorted).each { |key, n| raise 'RSS summary mismatch' unless row["cold_peak_increment_#{key}_bytes"] == n }
  end
  raise 'not fresh processes' unless pids.uniq.length == 15
  raise 'control truth mismatch' unless rows[-1]['samples'][0]['checksum'] == rows[-2]['samples'][0]['checksum']
  result = { validated_at: Time.now.iso8601, timing_samples: counts, cold_samples: pids.length, ambiguous_prepared: ambiguous, source_and_binary_hashes_match: true }
  File.open("#{PREFIX}-validation.json", File::WRONLY | File::CREAT | File::EXCL) { |f| f.puts JSON.pretty_generate(result) }
  puts JSON.generate(result)
end

stage = ARGV.fetch(0, 'all')
raise 'usage: collect-kfix-v2.rb [environment|action|kernel|render|memory|validate|all]' unless %w[environment action kernel render memory validate all].include?(stage)
environment if %w[environment all].include?(stage)
if %w[action all].include?(stage)
  measure("#{PREFIX}-action-timing.jsonl", [100, 1000].map { |n| ['target/release/examples/action_execution_bench', '--persons', n.to_s, '--seconds', '172800', '--warmups', '2', '--samples', '10', '--json'] })
end
if %w[kernel all].include?(stage)
  measure("#{PREFIX}-kernel-timing.jsonl", [
    %w[target/release/examples/kernel_bench --persons 100 --seconds 86400 --warmups 0 --samples 1 --json],
    %w[target/release/examples/kernel_bench --persons 100 --seconds 31536000 --warmups 2 --samples 10 --json]
  ])
end
if %w[render all].include?(stage)
  measure("#{PREFIX}-render-timing.jsonl", [%w[target/release/examples/render_snapshot_bench --persons 100 --warmups 2 --samples 10 --json]])
end
if %w[memory all].include?(stage)
  measure("#{PREFIX}-memory.jsonl", %w[action-100 action-1000 kernel-100-year render-control-100 render-snapshot-100].map { |name| ['target/release/palimpsest-bench-memory', '--run', name, '3'] })
end
validate if %w[validate all].include?(stage)
