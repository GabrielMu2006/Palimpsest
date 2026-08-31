extends SceneTree

const CaptureStatisticsClass = preload("res://capture_statistics.gd")

func _initialize() -> void:
	var records: Array = []
	for duration in range(1000, 21000, 1000):
		records.append({"frame_time_us": duration, "draw_calls": 1, "vram_bytes": 2})
	var result: Dictionary = CaptureStatisticsClass.summarize(records)
	_check(is_equal_approx(float(result["frame_time_us_p95"]), 19000.0), "p95 frame time uses sorted durations")
	_check(is_equal_approx(float(result["fps_p95"]), 500.0), "p95 FPS uses sorted reciprocal samples")
	_check(float(result["fps_p95"]) != 1_000_000.0 / float(result["frame_time_us_p95"]), "p95 FPS is not inverse p95 frame time")
	_check(CaptureStatisticsClass.summarize([{"frame_time_us": 0}]).is_empty(), "invalid interval rejects capture")
	if _failures.is_empty():
		print("capture statistics: ALL PASS")
	quit(_failures.size())

var _failures: Array[String] = []

func _check(condition: bool, what: String) -> void:
	if condition:
		print("ok: ", what)
	else:
		_failures.append(what)
		printerr("FAIL: ", what)
