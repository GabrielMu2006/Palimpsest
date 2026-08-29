extends Node

var bridge_ok: bool = false
var snapshot: Dictionary = {}
var bridge_benchmark_calls: int = 100_000
var bridge_baseline_usec: int = 0
var bridge_ping_usec: int = 0
var bridge_net_nanoseconds_per_call: float = 0.0
var bridge_sample_nanoseconds_per_call: Array[float] = []
var rendered_workload_complete: bool = false
var rendered_workload_result: Dictionary = {}

var _bridge: PalimpsestBridge

func _ready() -> void:
	_bridge = PalimpsestBridge.new()
	snapshot = _bridge.render_snapshot()
	bridge_ok = (
		snapshot.get("schema_version") == 1
		and snapshot.get("source") == "rust"
		and snapshot.get("example_entity_id") == 1
	)
	if not bridge_ok:
		push_error("Rust render snapshot bridge validation failed")
		return
	_run_bridge_benchmark(_bridge)


func _process(_delta: float) -> void:
	if rendered_workload_complete:
		return
	var tile_map = get_node("TileMap")
	if not tile_map.render_benchmark_complete:
		return
	rendered_workload_result = _bridge.benchmark_spike_workload(10_000, 1_000, 10)
	rendered_workload_complete = rendered_workload_result.get("ok", false)
	if not rendered_workload_complete:
		push_error("Rendered shared workload benchmark failed")


func _run_bridge_benchmark(bridge: PalimpsestBridge) -> void:
	var accumulator := 0
	for sample in 10:
		var started_usec := Time.get_ticks_usec()
		for value in bridge_benchmark_calls:
			accumulator = value
		bridge_baseline_usec = Time.get_ticks_usec() - started_usec

		started_usec = Time.get_ticks_usec()
		for value in bridge_benchmark_calls:
			accumulator = bridge.ping(value)
		bridge_ping_usec = Time.get_ticks_usec() - started_usec
		bridge_sample_nanoseconds_per_call.append(
			maxi(0, bridge_ping_usec - bridge_baseline_usec) * 1000.0
			/ bridge_benchmark_calls
		)
	bridge_sample_nanoseconds_per_call.sort()
	bridge_net_nanoseconds_per_call = bridge_sample_nanoseconds_per_call[5]
	if accumulator != bridge_benchmark_calls - 1:
		push_error("Rust ping bridge returned an unexpected value")
