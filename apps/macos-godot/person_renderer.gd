class_name PersonRenderer
extends MultiMeshInstance2D

## Draws up to 100 persons as one batched MultiMesh (a single draw call)
## purely from the snapshot's person arrays (CHRON-031, ADR-0026). This node
## holds no simulation state: every frame replaces the whole presentation
## mirror from the latest complete publication.

const TILE_SIZE := 4
const MAP_OFFSET := Vector2(24.0, 24.0)
const MAX_PERSONS := 100

## ADR-0026 action-state encoding: Idle=0, Moving=1, Eating=2, Sleeping=3,
## Working=4.
const STATE_COLORS: Array[Color] = [
	Color("9aa5a1"),  # Idle
	Color("f2f5f3"),  # Moving
	Color("7fd18a"),  # Eating
	Color("6f86d1"),  # Sleeping
	Color("d1a24f"),  # Working
]

var presented_count: int = 0


func _ready() -> void:
	position = Vector2.ZERO
	var quad := QuadMesh.new()
	quad.size = Vector2(TILE_SIZE - 1, TILE_SIZE - 1)
	var batch := MultiMesh.new()
	batch.transform_format = MultiMesh.TRANSFORM_2D
	batch.use_colors = true
	batch.mesh = quad
	batch.instance_count = 0
	multimesh = batch


## Rebuilds the whole presentation mirror from one snapshot frame.
func update_persons(frame: Dictionary) -> void:
	var xs: PackedInt32Array = frame["person_x"]
	var ys: PackedInt32Array = frame["person_y"]
	var states: PackedInt32Array = frame["person_state"]
	var count := xs.size()
	if ys.size() != count or states.size() != count:
		push_error("person arrays disagree on count")
		return
	var batch := multimesh
	batch.instance_count = count
	for i in count:
		var origin := MAP_OFFSET + Vector2(
			float(xs[i] * TILE_SIZE) + TILE_SIZE * 0.5,
			float(ys[i] * TILE_SIZE) + TILE_SIZE * 0.5
		)
		batch.set_instance_transform_2d(i, Transform2D(0.0, origin))
		var state := int(states[i])
		if state < 0 or state >= STATE_COLORS.size():
			state = 0
		batch.set_instance_color(i, STATE_COLORS[state])
	presented_count = count
