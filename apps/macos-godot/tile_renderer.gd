class_name MicroWorldTileRenderer
extends TileMapLayer

## Renders the 128×128 terrain batch from the Rust render snapshot
## (CHRON-031, ADR-0026). The tile layer is a presentation mirror: it is
## written only from snapshot bytes and never feeds values back into
## simulation.

const MAP_SIZE := 128
const TILE_SIZE := 4
const MAP_OFFSET := Vector2(24.0, 24.0)

## ADR-0026 terrain encoding: Ground=0, Water=1, Rock=2.
const TERRAIN_COLORS: Array[Color] = [
	Color("315d46"),
	Color("426b79"),
	Color("8c8155"),
]

var tile_count: int = 0
var terrain_applied: bool = false

var _source_id: int = -1


func _ready() -> void:
	position = MAP_OFFSET
	collision_enabled = false
	_build_tileset()


## Applies the snapshot terrain batch exactly once (terrain is static in
## Phase 1). Returns false and reports an error on a malformed batch.
func apply_terrain(cells: PackedByteArray) -> bool:
	if cells.size() != MAP_SIZE * MAP_SIZE:
		push_error("terrain batch has %d cells, expected %d" % [cells.size(), MAP_SIZE * MAP_SIZE])
		return false
	for y in MAP_SIZE:
		for x in MAP_SIZE:
			var kind := int(cells[y * MAP_SIZE + x])
			if kind < 0 or kind >= TERRAIN_COLORS.size():
				push_error("terrain cell (%d, %d) has unknown kind %d" % [x, y, kind])
				return false
			set_cell(Vector2i(x, y), _source_id, Vector2i(kind, 0))
	update_internals()
	tile_count = get_used_cells().size()
	terrain_applied = true
	return true


## Activity-site markers (Meal/Rest/Work) as simple presentation glyphs,
## rebuilt from each applied frame's site arrays.
func apply_sites(site_x: PackedInt32Array, site_y: PackedInt32Array, site_kind: PackedInt32Array) -> void:
	# Site markers live on the child canvas container.
	var markers := get_node("SiteMarkers")
	for child in markers.get_children():
		child.queue_free()
	for i in site_kind.size():
		var marker := ColorRect.new()
		marker.color = _site_color(int(site_kind[i]))
		marker.size = Vector2(TILE_SIZE, TILE_SIZE)
		marker.position = Vector2(site_x[i] * TILE_SIZE, site_y[i] * TILE_SIZE)
		markers.add_child(marker)


func _site_color(kind: int) -> Color:
	match kind:
		0:
			return Color("d1b55f")  # Meal
		1:
			return Color("7f6fd1")  # Rest
		2:
			return Color("c47f4a")  # Work
	push_error("unknown site kind %d" % kind)
	return Color.MAGENTA


func _build_tileset() -> void:
	var atlas_image := Image.create(TILE_SIZE * TERRAIN_COLORS.size(), TILE_SIZE, false, Image.FORMAT_RGBA8)
	for index in TERRAIN_COLORS.size():
		atlas_image.fill_rect(
			Rect2i(index * TILE_SIZE, 0, TILE_SIZE, TILE_SIZE), TERRAIN_COLORS[index]
		)

	var atlas := TileSetAtlasSource.new()
	atlas.texture = ImageTexture.create_from_image(atlas_image)
	atlas.texture_region_size = Vector2i(TILE_SIZE, TILE_SIZE)
	atlas.use_texture_padding = false
	for index in TERRAIN_COLORS.size():
		atlas.create_tile(Vector2i(index, 0))

	var runtime_tile_set := TileSet.new()
	runtime_tile_set.tile_size = Vector2i(TILE_SIZE, TILE_SIZE)
	_source_id = runtime_tile_set.add_source(atlas)
	tile_set = runtime_tile_set

	var markers := Node2D.new()
	markers.name = "SiteMarkers"
	add_child(markers)
