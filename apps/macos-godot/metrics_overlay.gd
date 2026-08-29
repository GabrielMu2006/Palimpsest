class_name DeveloperMetricsOverlay
extends PanelContainer

const UPDATE_INTERVAL_SECONDS := 0.25

var overlay_ready: bool = false
var visible_metrics_count: int = 0
var metrics_text: String = ""

var _elapsed_seconds := 0.0
var _metrics_label: Label


func _ready() -> void:
	_build_panel()
	_refresh_metrics()
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
	title.text = "PALIMPSEST  /  ARCHITECTURE SPIKE"
	title.add_theme_color_override("font_color", Color("88d1b5"))
	title.add_theme_font_size_override("font_size", 16)
	stack.add_child(title)

	var divider := HSeparator.new()
	stack.add_child(divider)

	_metrics_label = Label.new()
	_metrics_label.add_theme_color_override("font_color", Color("d7dfdc"))
	_metrics_label.add_theme_font_size_override("font_size", 14)
	_metrics_label.custom_minimum_size = Vector2(350.0, 0.0)
	stack.add_child(_metrics_label)


func _refresh_metrics() -> void:
	var main := get_node("../..")
	var tile_map := main.get_node("TileMap")
	var snapshot: Dictionary = main.snapshot
	var fps := Engine.get_frames_per_second()
	var video_memory_mib := (
		Performance.get_monitor(Performance.RENDER_VIDEO_MEM_USED) / 1024.0 / 1024.0
	)
	var bridge_status := "OK" if main.bridge_ok else "ERROR"
	var lines: Array[String] = [
		"MODE                 Rendered / presentation-only",
		"FPS                  %d" % fps,
		"FRAME PROCESS        %.3f ms" % (
			Performance.get_monitor(Performance.TIME_PROCESS) * 1000.0
		),
		"DRAW CALLS           %d" % int(
			Performance.get_monitor(Performance.RENDER_TOTAL_DRAW_CALLS_IN_FRAME)
		),
		"VIDEO MEMORY         %.2f MiB" % video_memory_mib,
		"TILES                %d / %d" % [tile_map.tile_count, 128 * 128],
		"TILE BENCHMARK       %s" % (
			"%.2f FPS" % tile_map.render_average_fps
			if tile_map.render_benchmark_complete else "sampling"
		),
		"RUST BRIDGE          %s" % bridge_status,
		"BRIDGE PING MEDIAN   %.2f ns" % main.bridge_net_nanoseconds_per_call,
		"RENDERED WORKLOAD    %s" % (
			"%.2f M work/s" % (
				main.rendered_workload_result.get("entity_work_per_second", 0.0)
				/ 1_000_000.0
			) if main.rendered_workload_complete else "pending"
		),
		"SNAPSHOT SCHEMA      v%d / %s" % [
			snapshot.get("schema_version", -1), snapshot.get("source", "unknown")
		],
		"SIM TIME             %d s" % snapshot.get("sim_second", -1),
		"ENTITY ID SAMPLE     %d" % snapshot.get("example_entity_id", -1),
		"SCHEDULER QUEUE      n/a (no client-owned simulation)",
	]
	visible_metrics_count = lines.size()
	metrics_text = "\n".join(lines)
	_metrics_label.text = metrics_text
