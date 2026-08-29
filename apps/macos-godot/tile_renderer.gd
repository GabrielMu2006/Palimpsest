class_name SpikeTileRenderer
extends TileMapLayer

const MAP_SIZE := 128
const TILE_SIZE := 4
const WARMUP_FRAMES := 60
const SAMPLE_FRAMES := 300

var tile_count: int = 0
var render_benchmark_complete: bool = false
var render_average_fps: float = 0.0
var render_minimum_fps: float = 0.0
var render_p95_frame_ms: float = 0.0

var _warmup_remaining := WARMUP_FRAMES
var _sample_elapsed_seconds := 0.0
var _sample_deltas: Array[float] = []


func _ready() -> void:
	position = Vector2(24.0, 24.0)
	collision_enabled = false
	_build_tile_map()


func _process(delta: float) -> void:
	if _warmup_remaining > 0:
		_warmup_remaining -= 1
		return
	if render_benchmark_complete:
		return
	_sample_elapsed_seconds += delta
	_sample_deltas.append(delta)
	if _sample_deltas.size() < SAMPLE_FRAMES:
		return

	_sample_deltas.sort()
	render_average_fps = SAMPLE_FRAMES / _sample_elapsed_seconds
	render_minimum_fps = 1.0 / _sample_deltas[-1]
	var p95_index := floori((_sample_deltas.size() - 1) * 0.95)
	render_p95_frame_ms = _sample_deltas[p95_index] * 1000.0
	render_benchmark_complete = true


func _build_tile_map() -> void:
	var atlas_image := Image.create(TILE_SIZE * 4, TILE_SIZE, false, Image.FORMAT_RGBA8)
	var colors: Array[Color] = [
		Color("315d46"),
		Color("477a52"),
		Color("8c8155"),
		Color("426b79"),
	]
	for atlas_x in colors.size():
		atlas_image.fill_rect(
			Rect2i(atlas_x * TILE_SIZE, 0, TILE_SIZE, TILE_SIZE),
			colors[atlas_x]
		)

	var atlas := TileSetAtlasSource.new()
	atlas.texture = ImageTexture.create_from_image(atlas_image)
	atlas.texture_region_size = Vector2i(TILE_SIZE, TILE_SIZE)
	atlas.use_texture_padding = false
	for atlas_x in colors.size():
		atlas.create_tile(Vector2i(atlas_x, 0))

	var runtime_tile_set := TileSet.new()
	runtime_tile_set.tile_size = Vector2i(TILE_SIZE, TILE_SIZE)
	var source_id := runtime_tile_set.add_source(atlas)
	tile_set = runtime_tile_set

	for y in MAP_SIZE:
		for x in MAP_SIZE:
			var terrain_index := (x / 11 + y / 17 + (x * y) / 97) % colors.size()
			set_cell(Vector2i(x, y), source_id, Vector2i(terrain_index, 0))
	update_internals()
	tile_count = get_used_cells().size()
