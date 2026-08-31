extends SceneTree

## CHRON-031 headless integration test (ADR-0026): snapshot fidelity, command
## acknowledgement semantics, time-control boundary behavior, bridge
## validation, and presentation authority. Run via:
##   gda script run tests/chron031_integration.gd --project apps/macos-godot --json

var _failures: Array[String] = []
const PersonRendererScript = preload("res://person_renderer.gd")
const TileRendererScript = preload("res://tile_renderer.gd")


func _initialize() -> void:
	call_deferred("_run")


func _check(condition: bool, what: String) -> void:
	if condition:
		print("ok: ", what)
	else:
		_failures.append(what)
		printerr("FAIL: ", what)


func _hex_id(bytes: PackedByteArray, index: int) -> String:
	var parts: Array[String] = []
	for offset in 8:
		parts.append("%02x" % bytes[index * 8 + offset])
	return "".join(parts)


func _wait_ack(world: PalimpsestMicroWorld, sequence: int) -> Dictionary:
	var deadline := Time.get_ticks_usec() + 5_000_000
	while Time.get_ticks_usec() < deadline:
		var status: Dictionary = world.command_status(sequence)
		if status.get("status") == "completed":
			return status
		await process_frame
	return {"status": "timeout"}


func _wait_for_sim_advance(world: PalimpsestMicroWorld, before: int) -> Dictionary:
	var deadline := Time.get_ticks_usec() + 5_000_000
	while Time.get_ticks_usec() < deadline:
		await process_frame
		var frame: Dictionary = world.snapshot_frame()
		if int(frame.get("sim_second", before)) > before:
			return frame
	return world.snapshot_frame()


func _persons_match(node: PersonRenderer, frame: Dictionary) -> bool:
	for i in frame["person_x"].size():
		var expected := Vector2(
			24.0 + float(frame["person_x"][i] * 4) + 2.0,
			24.0 + float(frame["person_y"][i] * 4) + 2.0
		)
		var expected_color: Color = node.STATE_COLORS[frame["person_state"][i]]
		if not node.multimesh.get_instance_transform_2d(i).origin.is_equal_approx(expected):
			return false
		if not node.multimesh.get_instance_color(i).is_equal_approx(expected_color):
			return false
	return true


func _run() -> void:
	var world := PalimpsestMicroWorld.new()
	var created: Dictionary = world.create_world("42", 100)
	_check(created.get("ok", false), "world created with seed 42 and 100 persons")
	_check(not world.create_world("42", 10).get("ok", true), "second create_world rejected")
	_check(not world.create_world("not-a-seed", 10).get("ok", true), "bad seed rejected")

	var frame0: Dictionary = world.snapshot_frame()
	_check(frame0.get("ok", false), "initial frame ok")
	_check(frame0["schema_version"] == 2, "schema version 2")
	_check(frame0["sim_second"] == 0, "initial sim_second is the epoch")
	_check(frame0["terrain"].size() == 16384, "terrain batch has 128x128 cells")
	_check(frame0["person_x"].size() == 100, "100 persons presented")
	_check(frame0["person_y"].size() == 100, "person y array matches")
	_check(frame0["person_id"].size() == 800, "100 lossless 8-byte ids")
	_check(frame0["person_action"].size() == 100, "person action array matches")
	_check(frame0["site_kind"].size() > 0, "activity sites present")
	var metrics0: Dictionary = frame0["metrics"]
	_check(metrics0["person_count"] == 100, "metrics person count mirrors snapshot")
	_check(frame0["worker"]["phase"] == 0, "worker starts paused")

	# Instantiate the same presentation nodes used by the main scene and verify
	# every mirrored value against the paused snapshot.
	var presentation := Node2D.new()
	get_root().add_child(presentation)
	var tile_node: MicroWorldTileRenderer = TileRendererScript.new()
	presentation.add_child(tile_node)
	var person_node: PersonRenderer = PersonRendererScript.new()
	presentation.add_child(person_node)
	await process_frame
	await process_frame
	_check(tile_node.apply_terrain(frame0["terrain"]), "terrain renderer accepts snapshot batch")
	tile_node.apply_sites(frame0["site_x"], frame0["site_y"], frame0["site_kind"])
	person_node.update_persons(frame0)
	_check(person_node.presented_count == 100 and person_node.multimesh.instance_count == 100, "person renderer applied all snapshot persons")
	var terrain_mirrored := true
	for y in 128:
		for x in 128:
			if tile_node.get_cell_atlas_coords(Vector2i(x, y)).x != int(frame0["terrain"][y * 128 + x]):
				terrain_mirrored = false
	_check(terrain_mirrored, "every terrain atlas coordinate mirrors snapshot")
	var sites := tile_node.get_node("SiteMarkers")
	var sites_mirrored: bool = sites.get_child_count() == frame0["site_kind"].size()
	if sites_mirrored:
		for i in sites.get_child_count():
			var marker: ColorRect = sites.get_child(i)
			var site_colors: Array[Color] = [Color("d1b55f"), Color("7f6fd1"), Color("c47f4a")]
			if marker.position != Vector2(frame0["site_x"][i] * 4, frame0["site_y"][i] * 4) or not marker.color.is_equal_approx(site_colors[frame0["site_kind"][i]]):
				sites_mirrored = false
	_check(sites_mirrored, "every site node coordinate mirrors snapshot")

	var seen_ids := {}
	var ids_valid := true
	var coords_valid := true
	for i in 100:
		var hex := _hex_id(frame0["person_id"], i)
		if hex == "0000000000000000" or seen_ids.has(hex):
			ids_valid = false
		seen_ids[hex] = true
		if frame0["person_x"][i] < 0 or frame0["person_x"][i] >= 128:
			coords_valid = false
		if frame0["person_y"][i] < 0 or frame0["person_y"][i] >= 128:
			coords_valid = false
		if frame0["person_state"][i] < 0 or frame0["person_state"][i] > 4:
			coords_valid = false
	_check(ids_valid, "person ids are non-zero, unique, byte-encoded")
	_check(coords_valid, "person coords and states in range")

	# Validation: nothing reaches the worker on rejection.
	_check(not world.command("set_speed", 7).get("ok", true), "invalid speed rejected at bridge")
	_check(not world.command("teleport", 0).get("ok", true), "unknown command rejected at bridge")
	_check(not world.command("step", -1).get("ok", true), "negative step rejected at bridge")

	# Time controls: exact paused step of 10 simulation seconds.
	var step := world.command("step", 10)
	_check(step.get("ok", false), "step enqueued")
	var ack := await _wait_ack(world, int(step["sequence"]))
	_check(ack.get("outcome") == "applied", "step ack applied")
	_check(ack.get("committed_to") == 10, "step committed exactly 10 sim seconds")
	var frame1: Dictionary = world.snapshot_frame()
	_check(frame1["sim_second"] == 10, "published snapshot at exactly 10s")
	_check(frame1["worker"]["phase"] == 0, "worker stays paused after step")
	person_node.update_persons(frame1)
	if DisplayServer.get_name() == "headless":
		print("person fidelity readback unavailable in headless renderer")
	else:
		_check(_persons_match(person_node, frame1), "windowed MultiMesh transforms and colors mirror stepped snapshot")
		person_node.multimesh.set_instance_transform_2d(0, Transform2D(0.0, Vector2(9999, 9999)))
		person_node.multimesh.set_instance_color(0, Color.MAGENTA)
		_check(not _persons_match(person_node, frame1), "windowed fidelity check detects corrupted presentation")
		person_node.update_persons(frame1)
		_check(_persons_match(person_node, frame1), "windowed presentation restores from snapshot")

	# Step while running is rejected by the worker (ack, not enqueue).
	var resume := world.command("resume", 0)
	_check(resume.get("ok", false), "resume enqueued")
	await _wait_ack(world, int(resume["sequence"]))
	var running_step := world.command("step", 5)
	_check(running_step.get("ok", false), "unpaused step enqueues (rejection is an ack)")
	var rejected := await _wait_ack(world, int(running_step["sequence"]))
	_check(String(rejected.get("outcome", "")).begins_with("rejected"), "unpaused step rejected")

	# Speed 1000x runs; pause freezes the presented boundary exactly.
	var speed := world.command("set_speed", 1000)
	_check(speed.get("ok", false), "set_speed 1000 enqueued")
	await _wait_ack(world, int(speed["sequence"]))
	var running_frame: Dictionary = await _wait_for_sim_advance(world, 10)
	_check(running_frame["sim_second"] > 10, "running worker advances beyond prior boundary")
	var pause := world.command("pause", 0)
	await _wait_ack(world, int(pause["sequence"]))
	var paused_a: Dictionary = world.snapshot_frame()
	for _i in 30:
		await process_frame
	var paused_b: Dictionary = world.snapshot_frame()
	_check(
		paused_a["sim_second"] == paused_b["sim_second"],
		"pause freezes the presented sim_second"
	)
	# Presentation edits/removal cannot affect the authoritative frame.
	person_node.multimesh.set_instance_transform_2d(0, Transform2D(0.0, Vector2(9999, 9999)))
	presentation.remove_child(tile_node)
	tile_node.queue_free()
	var after_node_edit: Dictionary = world.snapshot_frame()
	_check(after_node_edit["person_x"][0] == paused_b["person_x"][0], "editing/removing presentation nodes preserves Rust frame")
	presentation.queue_free()

	# Authority: mutating a presentation copy changes nothing upstream.
	var tampered := world.snapshot_frame()
	tampered["person_x"][0] = 99_999
	var reread: Dictionary = world.snapshot_frame()
	_check(reread["person_x"][0] != 99_999, "presentation mutation never reaches the snapshot")

	# Shutdown closes the command path.
	var shutdown := world.command("shutdown", 0)
	_check(shutdown.get("ok", false), "shutdown enqueued")
	await _wait_ack(world, int(shutdown["sequence"]))
	for _i in 60:
		await process_frame
	_check(world.snapshot_frame()["worker"]["phase"] == 3, "worker closed after shutdown")
	_check(not world.command("pause", 0).get("ok", true), "closed worker rejects commands")

	if _failures.is_empty():
		print("CHRON-031 integration: ALL PASS")
	else:
		printerr("CHRON-031 integration: %d failures" % _failures.size())
	quit(_failures.size())
