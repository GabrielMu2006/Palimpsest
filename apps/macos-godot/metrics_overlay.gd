class_name DeveloperMetricsOverlay
extends PanelContainer

## Read-only metrics overlay (CHRON-031): the SIMULATION section mirrors the
## Rust snapshot/worker metrics verbatim; the CLIENT section is labelled
## client-side rendering data; fields nobody provides are labelled
## "unavailable", never fabricated (ADR-0026 §3).

const UPDATE_INTERVAL_SECONDS := 0.25

var overlay_ready: bool = false
var visible_metrics_count: int = 0
var metrics_text: String = ""

var _elapsed_seconds := 0.0
var _metrics_label: Label


func _ready() -> void:
	_build_panel()
	overlay_ready = true


func _process(delta: float) -> void:
	_elapsed_seconds += delta
	if _elapsed_seconds < UPDATE_INTERVAL_SECONDS:
		return
	_elapsed_seconds = 0.0
	_refresh_metrics()


func _build_panel() -> void:
	var panel_style := StyleBoxFlat.new()
	panel_style.bg_color = Color(0.035, 0.043, 0.055, 0.94)
	panel_style.border_color = Color(0.27, 0.48, 0.43, 1.0)
	panel_style.set_border_width_all(1)
	panel_style.set_corner_radius_all(4)
	add_theme_stylebox_override("panel", panel_style)

	var margin := MarginContainer.new()
	margin.add_theme_constant_override("margin_left", 16)
	margin.add_theme_constant_override("margin_top", 14)
	margin.add_theme_constant_override("margin_right", 16)
	margin.add_theme_constant_override("margin_bottom", 14)
	add_child(margin)

	var stack := VBoxContainer.new()
	stack.add_theme_constant_override("separation", 8)
	margin.add_child(stack)

	var title := Label.new()
	title.text = "PALIMPSEST / MICRO WORLD (Phase 1)"
	title.add_theme_color_override("font_color", Color("88d1b5"))
	title.add_theme_font_size_override("font_size", 16)
	stack.add_child(title)

	var divider := HSeparator.new()
	stack.add_child(divider)

	_metrics_label = Label.new()
	_metrics_label.add_theme_color_override("font_color", Color("d7dfdc"))
	_metrics_label.add_theme_font_size_override("font_size", 13)
	_metrics_label.custom_minimum_size = Vector2(430.0, 0.0)
	stack.add_child(_metrics_label)


func _refresh_metrics() -> void:
	var main := get_node("../..")
	var frame: Dictionary = main.latest_frame
	if frame.is_empty():
		return
	var metrics: Dictionary = frame["metrics"]
	var worker: Dictionary = frame["worker"]
	var phase_names := ["PAUSED", "RUNNING", "FAULTED", "CLOSED"]
	var phase: int = worker["phase"]
	var speed: int = worker["speed"]
	var speed_text := "MAX" if speed == 0 else "%dx" % speed
	var lines: Array[String] = [
		"SIMULATION (snapshot v%d, publication %d)" % [frame["schema_version"], frame["publications"]],
		"  SIM TIME           %d s" % frame["sim_second"],
		"  PHASE / SPEED      %s / %s" % [phase_names[phase], speed_text],
		"  PERSONS            %d (live actions %d)" % [
			metrics["person_count"], metrics["live_actions"]
		],
		"  SCHEDULER QUEUE    %d" % metrics["scheduler_queue_depth"],
		"  EVENTS             committed %d, buffered %d, rotated %d" % [
			metrics["events_committed"], metrics["events_buffered"],
			metrics["buffer_rotations"]
		],
		"  ROUNDS/TRANS/DEC   %d / %d / %d" % [
			metrics["rounds_total"], metrics["transitions_total"],
			metrics["decisions_total"]
		],
		"  COMMANDS           applied %d, rejected %d, queue %d (max %d)" % [
			worker["commands_applied"], worker["commands_rejected"],
			worker["queue_depth"], worker["max_queue_depth"]
		],
		"  LOD DISTRIBUTION   unavailable (no LOD system in Phase 1)",
		"CLIENT (rendering only, not simulation truth)",
		"  FPS                %d" % Engine.get_frames_per_second(),
		"  FRAME PROCESS      %.3f ms" % (
			Performance.get_monitor(Performance.TIME_PROCESS) * 1000.0
		),
		"  DRAW CALLS         %d" % int(
			Performance.get_monitor(Performance.RENDER_TOTAL_DRAW_CALLS_IN_FRAME)
		),
		"  VIDEO MEMORY       %.2f MiB" % (
			Performance.get_monitor(Performance.RENDER_VIDEO_MEM_USED) / 1024.0 / 1024.0
		),
	]
	visible_metrics_count = lines.size()
	metrics_text = "\n".join(lines)
	_metrics_label.text = metrics_text
