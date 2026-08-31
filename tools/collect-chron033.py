#!/usr/bin/env python3
"""Run already-built reference binaries sequentially; keep failures and source hashes."""
import hashlib,json,pathlib,subprocess,time
ROOT=pathlib.Path(__file__).resolve().parents[1]
OUT=ROOT/'docs/reports/data'

def invoke(name,cmd,timeout,stdout_suffix=".json"):
    start=time.monotonic()
    with (OUT/(name+stdout_suffix)).open('w')as out,(OUT/(name+'.stderr.txt')).open('w')as err:
        try:
            result=subprocess.run(cmd,cwd=ROOT,stdout=out,stderr=err,timeout=timeout)
            status={'exit_code':result.returncode}
        except subprocess.TimeoutExpired:
            status={'timeout_seconds':timeout}
    status.update(command=cmd,wall_seconds=time.monotonic()-start)
    (OUT/(name+'.invocation.json')).write_text(json.dumps(status,indent=2)+'\n')
    print(name,status,flush=True)
    return status.get('exit_code')==0

if __name__=='__main__':
    sources=[p for top in ('crates','apps','tools')for p in (ROOT/top).rglob('*')if p.is_file()and p.suffix in ('.rs','.toml','.gd','.tscn','.py')and not any(x in p.parts for x in ('target','.godot','bin'))]
    # Explicit binaries and bin sources excluded above are included here.
    sources+=list((ROOT/'apps/headless-runner/src/bin').glob('*.rs'))+list((ROOT/'tools/bench-memory/src/bin').glob('*.rs'))
    sources +=[ROOT/'Cargo.lock',ROOT/'Cargo.toml',ROOT/'MASTER_SPEC.md']
    sources +=[ROOT/'target/release'/name for name in ('bench_micro_world','bench_micro_worker','micro_memory','libpalimpsest_godot_bridge.dylib')]
    identity={'head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),'source':'dirty reviewed candidate; hash manifest is source identity','sha256':{str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest()for p in sources}}
    (OUT/'chron-033-source.json').write_text(json.dumps(identity,indent=2)+'\n')
    for scale in (100,1000,3000,5000,10000):
        ok=invoke(f'chron-033-scale-{scale}',[str(ROOT/'target/release/bench_micro_world'),'--scales',str(scale),'--seconds','86400','--warmups','2','--samples','10'],1800)
        # Failed larger fixture is retained; RSS attempt remains visible too.
        invoke(f'chron-033-rss-{scale}',[str(ROOT/'target/release/micro_memory'),str(scale)],300)
        if scale==100 and not ok:raise SystemExit('Mandatory 100-person scale failed')
    if not invoke('chron-033-worker',[str(ROOT/'target/release/bench_micro_worker')],300):raise SystemExit('Worker comparison failed')
    cmd=['/usr/bin/time','-l','/Users/gabrielmu/Applications/Godot.app/Contents/MacOS/Godot','--path',str(ROOT/'apps/macos-godot'),'--script','res://tests/chron033_rendered_compare.gd','--','--output='+str(OUT/'chron-033-rendered.json')]
    # Godot writes JSON directly; stdout is an engine log.
    if not invoke('chron-033-rendered-engine',cmd,300,'.stdout.txt'):raise SystemExit('Windowed comparison failed')
