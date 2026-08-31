extends Node

## Palimpsest Phase 1 micro-world presentation (CHRON-031, ADR-0026).
##
## This node owns no simulation state: it creates the Rust worker once,
## reads one batched snapshot frame per rendered frame, mirrors it into
## presentation nodes, and forwards time-control intents as worker commands.
## `--capture-json=<path>` (user arg) runs the windowed frame-capture
## benchmark: resume at 100x, discard 120 warm-up frames, measure 300 frames,
## write JSON, quit.

const WORLD_SEED := "42"
const WORLD_PERSONS := 100
const CAPTURE_WARMUP_FRAMES := 120
const CAPTURE_MEASURED_FRAMES := 300
const CaptureStatisticsScript = preload("res://capture_statistics.gd")

var latest_frame: Dictionary = {}
var world_ready: bool = false

var _world: PalimpsestMicroWorld
var _tile_map: MicroWorldTileRenderer
var _persons: PersonRenderer
var _controls: TimeControls
var _pending_commands: Dictionary = {}

# Frame-capture state (windowed benchmark path only).
var _capture_path: String = ""
var _capture_warmup_remaining: int = 0
var _capture_records: Array[Dictionary] = []
var _capture_last_timestamp_usec: int = 0
var _capture_finished: bool = false


func _ready() -> void:
	_world = PalimpsestMicroWorld.new()
	var created: Dictionary = _world.create_world(WORLD_SEED, WORLD_PERSONS)
	if not created.get("ok", false):
		push_error("micro world creation failed: %s" % created.get("error", "unknown"))
		get_tree().quit(1)
		return
	_tile_map = get_node("TileMap")
	_persons = get_node("Persons")
	_controls = get_node("DeveloperUI/TimeControls")
	_controls.pause_requested.connect(func() -> void: _send("pause", 0))
	_controls.resume_requested.connect(func() -> void: _send("resume", 0))
	_controls.step_requested.connect(func(steps: int) -> void: _send("step", steps))
	_controls.speed_requested.connect(func(speed: int) -> void: _send("set_speed", speed))
	world_ready = true
	_start_capture_if_requested()


func _process(_delta: float) -> void:
	if not world_ready:
		return
	var frame_start_usec := Time.get_ticks_usec()
	var frame: Dictionary = _world.snapshot_frame()
	var snapshot_duration_usec := Time.get_ticks_usec() - frame_start_usec
	if not frame.get("ok", false):
		push_error("snapshot frame failed: %s" % frame.get("error", "unknown"))
		world_ready = false
		if not _capture_path.is_empty():
			get_tree().quit(1)
		return
	latest_frame = frame
	var node_start_usec := Time.get_ticks_usec()
	if not _tile_map.terrain_applied:
		if _tile_map.apply_terrain(frame["terrain"]):
			_tile_map.apply_sites(frame["site_x"], frame["site_y"], frame["site_kind"])
	_persons.update_persons(frame)
	var node_update_duration_usec := Time.get_ticks_usec() - node_start_usec
	_poll_command_acks()
	_capture_frame(frame_start_usec, snapshot_duration_usec, node_update_duration_usec)


## Issues one worker command; enqueue success and the later acknowledgement
## are distinct UI states.
func _send(command_type: String, value: int) -> void:
	var result: Dictionary = _world.command(command_type, value)
	if result.get("ok", false):
		var sequence := int(result["sequence"])
		_pending_commands[sequence] = command_type
		_controls.show_enqueue(true, "%d (%s)" % [sequence, command_type])
	else:
		_controls.show_enqueue(false, str(result.get("error", "unknown")))


func _poll_command_acks() -> void:
	for sequence in _pending_commands.keys():
		var status: Dictionary = _world.command_status(sequence)
		match status.get("status", "unknown"):
			"completed":
				_controls.show_outcome(
					status.get("outcome", "?"), int(status.get("committed_to", -1))
				)
				_pending_commands.erase(sequence)
			"evicted", "unknown":
				_pending_commands.erase(sequence)
			_:
				pass  # still pending


func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventKey and event.pressed and not event.echo:
		match event.physical_keycode:
			KEY_SPACE:
				var worker: Dictionary = latest_frame.get("worker", {})
				if int(worker.get("phase", 0)) == 0:
					_send("resume", 0)
				else:
					_send("pause", 0)
			KEY_PERIOD:
				_send("step", 1)
			KEY_1:
				_send("set_speed", 1)
			KEY_2:
				_send("set_speed", 5)
			KEY_3:
				_send("set_speed", 20)
			KEY_4:
				_send("set_speed", 100)
			KEY_5:
				_send("set_speed", 1000)
			KEY_6:
				_send("set_speed", 0)


func _start_capture_if_requested() -> void:
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--capture-json="):
			_capture_path = arg.trim_prefix("--capture-json=")
		elif arg == "--capture-minimal":
			# Base-terrain isolation: hide UI and persons so draw calls reflect
			# the tile map alone (P1-REMAINING §4 separates the two).
			get_node("DeveloperUI").visible = false
			get_node("Persons").visible = false
			get_node("TileMap/SiteMarkers").visible = false
	if _capture_path.is_empty():
		return
	# Drive the world at 100x so persons visibly move during the capture.
	_send("set_speed", 100)
	_send("resume", 0)
	_capture_warmup_remaining = CAPTURE_WARMUP_FRAMES
	_capture_last_timestamp_usec = 0


func _capture_frame(timestamp_usec: int, snapshot_duration_usec: int, node_update_duration_usec: int) -> void:
	if _capture_path.is_empty() or _capture_finished:
		return
	var frame_time_usec := 0
	if _capture_last_timestamp_usec > 0:
		frame_time_usec = timestamp_usec - _capture_last_timestamp_usec
	_capture_last_timestamp_usec = timestamp_usec
	if _capture_warmup_remaining > 0:
		_capture_warmup_remaining -= 1
		return
	if frame_time_usec <= 0:
		push_error("nonpositive raw frame interval")
		get_tree().quit(1)
		return
	var record := {
		"timestamp_usec": timestamp_usec,
		"frame_time_us": frame_time_usec,
		"snapshot_frame_duration_us": snapshot_duration_usec,
		"node_update_duration_us": node_update_duration_usec,
		"sim_second": int(latest_frame.get("sim_second", -1)),
		"publication": int(latest_frame.get("publications", -1)),
		"draw_calls": int(Performance.get_monitor(Performance.RENDER_TOTAL_DRAW_CALLS_IN_FRAME)),
		"vram_bytes": int(Performance.get_monitor(Performance.RENDER_VIDEO_MEM_USED)),
	}
	for key in ["snapshot_age_us", "snapshot_build_us", "bridge_conversion_us"]:
		if latest_frame.has(key):
			record[key] = latest_frame[key]
	_capture_records.append(record)
	if _capture_records.size() < CAPTURE_MEASURED_FRAMES:
		return
	_write_capture_report()


func _write_capture_report() -> void:
	_capture_finished = true
	var stats: Dictionary = CaptureStatisticsScript.summarize(_capture_records)
	var report := {
		"task": "CHRON-031",
		"persons": WORLD_PERSONS,
		"seed": WORLD_SEED,
		"speed": "100x",
		"warmup_frames": CAPTURE_WARMUP_FRAMES,
		"measured_frames": _capture_records.size(),
		"measurement_method": "Time.get_ticks_usec monotonic frame-start timestamps; raw consecutive records after warm-up",
		"measurement_window_usec": int(_capture_records[-1]["timestamp_usec"]) - int(_capture_records[0]["timestamp_usec"]) + int(_capture_records[0]["frame_time_us"]),
		"sim_second_end": latest_frame.get("sim_second", -1),
		"publications_end": latest_frame.get("publications", -1),
	}
	report.merge(stats)
	report["frame_ms_mean"] = float(stats["frame_time_us_mean"]) / 1000.0
	report["frame_ms_p95"] = float(stats["frame_time_us_p95"]) / 1000.0
	report["frame_ms_max"] = float(stats["frame_time_us_max"]) / 1000.0
	report["video_memory_bytes_p95"] = stats["vram_bytes_p95"]
	report["records"] = _capture_records
	var file := FileAccess.open(_capture_path, FileAccess.WRITE)
	if file == null:
		push_error("cannot write capture report to %s" % _capture_path)
		get_tree().quit(1)
	else:
		file.store_string(JSON.stringify(report, "  "))
		file.close()
		get_tree().quit(0)
