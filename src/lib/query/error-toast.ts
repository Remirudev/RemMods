import { notifications } from '@mantine/notifications'
import { describeError } from '../i18n/error-messages'

export function showErrorToast(error: unknown): void {
  const { title, message } = describeError(error)
  notifications.show({ color: 'red', title, message, autoClose: 6000 })
}
