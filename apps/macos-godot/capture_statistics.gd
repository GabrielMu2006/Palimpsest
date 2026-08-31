class_name CaptureStatistics
extends RefCounted

## Pure capture aggregation. Records contain integer microsecond durations.
## FPS percentiles are computed from sorted reciprocal samples, independently
## from frame-time percentiles.

static func percentile(values: Array, fraction: float) -> float:
	if values.is_empty():
		return 0.0
	var sorted: Array = values.duplicate()
	sorted.sort()
	var index := int(floor(float(sorted.size() - 1) * fraction))
	return float(sorted[index])

static func summarize(records: Array) -> Dictionary:
	var frame_us: Array = []
	var fps: Array = []
	var draw_calls: Array[int] = []
	var vram: Array[int] = []
	var snapshot_us: Array = []
	var node_us: Array = []
	var age_us: Array = []
	var build_us: Array = []
	var conversion_us: Array = []
	for record: Dictionary in records:
		var duration := int(record.get("frame_time_us", 0))
		if duration <= 0:
			return {}
		frame_us.append(duration)
		fps.append(1_000_000.0 / float(duration))
		draw_calls.append(int(record.get("draw_calls", 0)))
		vram.append(int(record.get("vram_bytes", 0)))
		for pair in [["snapshot_frame_duration_us", snapshot_us], ["node_update_duration_us", node_us], ["snapshot_age_us", age_us], ["snapshot_build_us", build_us], ["bridge_conversion_us", conversion_us]]:
			if record.has(pair[0]):
				pair[1].append(int(record[pair[0]]))
	if frame_us.is_empty():
		return {}
	var frame_mean := 0.0
	var draw_mean := 0.0
	for value: int in frame_us:
		frame_mean += float(value)
	frame_mean /= float(frame_us.size())
	for value: int in draw_calls:
		draw_mean += float(value)
	draw_mean /= float(draw_calls.size())
	var fps_mean := 0.0
	if frame_mean > 0.0:
		fps_mean = 1_000_000.0 / frame_mean
	var result := {
		"fps_min": percentile(fps, 0.0),
		"fps_mean": fps_mean,
		"fps_p95": percentile(fps, 0.95),
		"frame_time_us_mean": frame_mean,
		"frame_time_us_p95": percentile(frame_us, 0.95),
		"frame_time_us_max": percentile(frame_us, 1.0),
		"draw_calls_min": percentile(draw_calls, 0.0),
		"draw_calls_mean": draw_mean,
		"draw_calls_p95": percentile(draw_calls, 0.95),
		"vram_bytes_p95": percentile(vram, 0.95),
	}
	for pair in [["snapshot_frame_duration_us", snapshot_us], ["node_update_duration_us", node_us], ["snapshot_age_us", age_us], ["snapshot_build_us", build_us], ["bridge_conversion_us", conversion_us]]:
		if not pair[1].is_empty():
			var sum := 0.0
			for value: int in pair[1]:
				sum += float(value)
			result[pair[0] + "_mean"] = sum / float(pair[1].size())
			result[pair[0] + "_p95"] = percentile(pair[1], 0.95)
	return result
