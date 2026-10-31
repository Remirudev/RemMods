export type UiScaleAction = 'increase' | 'decrease' | 'reset'

export type UiScaleKeyEvent = Pick<KeyboardEvent, 'key' | 'ctrlKey' | 'altKey' | 'metaKey'>
