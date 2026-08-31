extends SceneTree

## CHRON-033 rendered/direct same-work comparison. This is a windowed-only
## harness: each sample gets a fresh Main scene and worker, then advances the
## identical fixture to one explicit boundary (never via 1x pacing).

const TARGET_PERSONS := 100
const TARGET_SEED := "42"
const DEFAULT_SECONDS := 86400
const ENGINE_WARMUP_FRAMES := 120
const DEFAULT_SIM_WARMUPS := 2
const DEFAULT_SAMPLES := 10
const TIMEOUT_USEC := 120_000_000

var _failures: Array[String] = []
var _output_path := ""
var _seconds := DEFAULT_SECONDS
var _sim_warmups := DEFAULT_SIM_WARMUPS
var _samples := DEFAULT_SAMPLES
var _records: Array[Dictionary] = []
var _baseline_hash := ""
var _baseline_work := ""
var _seen_args := {}

func _initialize() -> void:
	call_deferred("_run")

func _fail(message: String) -> void:
	_failures.append(message)
	printerr("FAIL: ", message)

func _parse_args() -> bool:
	for arg in OS.get_cmdline_user_args():
		if _seen_args.has(arg.split("=", true, 1)[0]):
			_fail("duplicate argument: %s" % arg)
			return false
		if arg.begins_with("--output="):
			_output_path = arg.trim_prefix("--output=")
		elif arg.begins_with("--samples="):
			_samples = int(arg.trim_prefix("--samples="))
		elif arg.begins_with("--warmups="):
			_sim_warmups = int(arg.trim_prefix("--warmups="))
		elif arg.begins_with("--seconds="):
			_seconds = int(arg.trim_prefix("--seconds="))
		else:
			_fail("unknown argument: %s" % arg)
			return false
		_seen_args[arg.split("=", true, 1)[0]] = true
	if _output_path.is_empty() or not _output_path.begins_with("/") or _samples <= 0 or _sim_warmups < 0 or _seconds <= 0:
		_fail("absolute --output and positive --samples/--seconds are required")
		return false
	for arg in OS.get_cmdline_user_args():
		var key := arg.split("=", true, 1)[0]
		var value := arg.get_slice("=", 1)
		if key != "--output" and (value.is_empty() or not value.is_valid_int() or int(value) < 0 or (key != "--warmups" and int(value) <= 0)):
			_fail("invalid integer argument: %s" % arg)
			return false
	return true

func _wait_ack(world: PalimpsestMicroWorld, sequence: int) -> Dictionary:
	var deadline := Time.get_ticks_usec() + TIMEOUT_USEC
	while Time.get_ticks_usec() < deadline:
		var status: Dictionary = world.command_status(sequence)
		if status.get("status") == "completed":
			return status
		await process_frame
	return {"status": "timeout"}

func _new_scene() -> Node:
	var scene: Node = load("res://main.tscn").instantiate()
	get_root().add_child(scene)
	await RenderingServer.frame_post_draw
	return scene

func _close_scene(scene: Node) -> bool:
	var clean := true
	var world: PalimpsestMicroWorld = scene._world
	if world != null:
		var shutdown: Dictionary = world.command("shutdown", 0)
		if shutdown.get("ok", false):
			var status: Dictionary = await _wait_ack(world, int(shutdown["sequence"]))
			clean = status.get("outcome") == "applied"
		else:
			clean = false
		scene.world_ready = false
		scene._world = null
	scene.queue_free()
	await process_frame
	if not clean:
		_fail("scene shutdown was not acknowledged as applied")
	return clean

func _work_fingerprint(frame: Dictionary) -> String:
	var metrics: Dictionary = frame.get("metrics", {})
	return JSON.stringify({
		"rounds_total": metrics.get("rounds_total", -1),
		"transitions_total": metrics.get("transitions_total", -1),
		"decisions_total": metrics.get("decisions_total", -1),
		"events_committed": metrics.get("events_committed", -1),
		"person_count": metrics.get("person_count", -1),
	})

func _measure_one(kind: String, index: int) -> Dictionary:
	var scene: Node = await _new_scene()
	var world: PalimpsestMicroWorld = scene._world
	var initial: Dictionary = scene.latest_frame
	if int(initial.get("sim_second", -1)) != 0 or int(initial.get("metrics", {}).get("person_count", -1)) != TARGET_PERSONS or int(initial.get("worker", {}).get("phase", -1)) != 0:
		_fail("%s %d scene fixture is not 100-person paused epoch" % [kind, index])
		await _close_scene(scene)
		return {}
	var frames_before_draw := Engine.get_frames_drawn()
	var command_started := Time.get_ticks_usec()
	var command: Dictionary = world.command("advance_to", _seconds)
	if not command.get("ok", false):
		_fail("%s %d AdvanceTo enqueue failed: %s" % [kind, index, command.get("error", "unknown")])
		await _close_scene(scene)
		return {}
	var ack: Dictionary = await _wait_ack(world, int(command["sequence"]))
	var ack_observed := Time.get_ticks_usec()
	if ack.get("outcome") != "applied" or int(ack.get("committed_to", -1)) != _seconds:
		_fail("%s %d AdvanceTo acknowledgement invalid: %s" % [kind, index, ack])
		await _close_scene(scene)
		return {}
	var deadline := Time.get_ticks_usec() + TIMEOUT_USEC
	while int(scene.latest_frame.get("sim_second", -1)) != _seconds and Time.get_ticks_usec() < deadline:
		await process_frame
	if int(scene.latest_frame.get("sim_second", -1)) != _seconds:
		_fail("%s %d final frame was not drawn at target boundary" % [kind, index])
		await _close_scene(scene)
		return {}
	await RenderingServer.frame_post_draw
	var final_drawn := Time.get_ticks_usec()
	var frame: Dictionary = scene.latest_frame
	var snapshot_started := Time.get_ticks_usec()
	var final_read: Dictionary = world.snapshot_frame()
	var snapshot_call_us := Time.get_ticks_usec() - snapshot_started
	var hash := String(world.snapshot_diagnostic_hash())
	if not final_read.get("ok", false) or hash.is_empty():
		_fail("%s %d final snapshot/hash unavailable" % [kind, index])
	var work := _work_fingerprint(frame)
	if _baseline_hash.is_empty():
		_baseline_hash = hash
		_baseline_work = work
	else:
		if hash != _baseline_hash:
			_fail("%s %d diagnostic hash differs" % [kind, index])
		if work != _baseline_work:
			_fail("%s %d work counters differ" % [kind, index])
	var result := {
		"kind": kind,
		"index": index,
		"advance_seconds": _seconds,
		"submit_to_ack_observed_us": ack_observed - command_started,
		"ack_to_final_frame_drawn_us": final_drawn - ack_observed,
		"submit_to_final_frame_drawn_us": final_drawn - command_started,
		"frames_drawn": Engine.get_frames_drawn() - frames_before_draw,
		"final_snapshot_call_us": snapshot_call_us,
		"final_snapshot_age_us": final_read.get("snapshot_age_us", null),
		"final_snapshot_build_us": final_read.get("snapshot_build_us", null),
		"final_bridge_conversion_us": final_read.get("bridge_conversion_us", null),
		"final_frame_drawn_timestamp_usec": final_drawn,
		"final_hash": hash,
		"work": JSON.parse_string(work),
		"sim_second": frame.get("sim_second", -1),
		"publications": frame.get("publications", -1),
		"metrics": frame.get("metrics", {}),
	}
	await _close_scene(scene)
	return result

func _timing_summary() -> Dictionary:
	var values: Array = []
	var summary := {}
	for row in _records:
		if row.get("kind") == "timed":
			values.append(float(row.get("submit_to_ack_observed_us", 0)))
	if values.is_empty():
		return {"timed_record_count": 0}
	var sorted: Array = values.duplicate()
	sorted.sort()
	var mean := 0.0
	for value: float in values:
		mean += value
	mean /= values.size()
	var variance := 0.0
	for value: float in values:
		variance += (value - mean) * (value - mean)
	summary = {"timed_record_count": values.size(), "submit_to_ack_us_min": sorted[0], "submit_to_ack_us_median": sorted[int(sorted.size() / 2)], "submit_to_ack_us_max": sorted[-1], "submit_to_ack_us_mean": mean, "submit_to_ack_us_variance": variance / values.size()}
	for key in ["submit_to_final_frame_drawn_us", "ack_to_final_frame_drawn_us", "final_snapshot_call_us", "final_snapshot_age_us", "final_snapshot_build_us", "final_bridge_conversion_us"]:
		var samples: Array = []
		for row in _records:
			if row.get("kind") == "timed" and row.get(key) != null:
				samples.append(float(row[key]))
		if not samples.is_empty():
			samples.sort()
			summary[key + "_median"] = samples[int(samples.size() / 2)]
	return summary

func _run() -> void:
	if not _parse_args() or DisplayServer.get_name() == "headless":
		if DisplayServer.get_name() == "headless":
			_fail("windowed renderer required; headless comparison rejected")
		quit(2)
		return
	# One engine warmup is intentionally outside simulation samples.
	var warm_scene: Node = await _new_scene()
	for _i in ENGINE_WARMUP_FRAMES:
		await process_frame
	await _close_scene(warm_scene)
	for i in _sim_warmups:
		var result: Dictionary = await _measure_one("simulation_warmup", i)
		if not result.is_empty():
			_records.append(result)
		else:
			break
	if not _failures.is_empty():
		_write_report()
		quit(1)
		return
	for i in _samples:
		var result: Dictionary = await _measure_one("timed", i)
		if not result.is_empty():
			_records.append(result)
		else:
			break
		if not _failures.is_empty():
			break
	_write_report()
	quit(0 if _failures.is_empty() else 1)

func _write_report() -> void:
	var file := FileAccess.open(_output_path, FileAccess.WRITE)
	if file == null:
		_fail("cannot write output: %s" % _output_path)
		return
	file.store_string(JSON.stringify({
		"schema": 1, "task": "CHRON-033",
		"method": "windowed fresh Main scene per run; monotonic AdvanceTo acknowledgement and frame_post_draw timestamps",
		"seed": TARGET_SEED, "persons": TARGET_PERSONS, "seconds": _seconds,
		"engine_warmup_frames": ENGINE_WARMUP_FRAMES, "simulation_warmups": _sim_warmups, "samples": _samples,
		"summary": _timing_summary(), "failures": _failures, "records": _records,
	}, "  "))
	file.close()
	if _failures.is_empty():
		print("CHRON-033 rendered comparison: ALL PASS")

	return
