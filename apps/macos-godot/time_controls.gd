class_name TimeControls
extends HBoxContainer

## Minimal time controls (CHRON-031): every button only issues a worker
## command through the bridge; enqueue success and the later applied/rejected
## acknowledgement are distinct UI states (ADR-0026 §2). No button mutates
## world state directly.

signal pause_requested
signal resume_requested
signal step_requested(steps: int)
signal speed_requested(speed: int)

var last_enqueue: String = "none"
var last_outcome: String = "none"

var _feedback_label: Label
var _speed_options: OptionButton


func _ready() -> void:
	add_theme_constant_override("separation", 6)

	var pause_button := _make_button("Pause")
	pause_button.pressed.connect(func() -> void: pause_requested.emit())
	var resume_button := _make_button("Resume")
	resume_button.pressed.connect(func() -> void: resume_requested.emit())
	var step1_button := _make_button("Step +1s")
	step1_button.pressed.connect(func() -> void: step_requested.emit(1))
	var step10_button := _make_button("Step +10s")
	step10_button.pressed.connect(func() -> void: step_requested.emit(10))

	_speed_options = OptionButton.new()
	for entry in [[1, "1x"], [5, "5x"], [20, "20x"], [100, "100x"], [1000, "1000x"], [0, "MAX"]]:
		_speed_options.add_item(entry[1], entry[0])
	_speed_options.selected = 0
	_speed_options.item_selected.connect(
		func(index: int) -> void: speed_requested.emit(_speed_options.get_item_id(index))
	)
	add_child(_speed_options)

	_feedback_label = Label.new()
	_feedback_label.add_theme_font_size_override("font_size", 12)
	add_child(_feedback_label)


## Enqueue feedback (distinct from the later applied/rejected state).
func show_enqueue(ok: bool, detail: String) -> void:
	last_enqueue = "enqueued #%s" % detail if ok else "ENQUEUE FAILED: %s" % detail
	_update_feedback()


## Application acknowledgement feedback from the ack poll.
func show_outcome(outcome: String, committed_to: int) -> void:
	last_outcome = "%s @ %ds" % [outcome, committed_to]
	_update_feedback()


func _update_feedback() -> void:
	_feedback_label.text = "cmd: %s | ack: %s" % [last_enqueue, last_outcome]


func _make_button(label: String) -> Button:
	var button := Button.new()
	button.text = label
	button.focus_mode = Control.FOCUS_NONE
	add_child(button)
	return button
