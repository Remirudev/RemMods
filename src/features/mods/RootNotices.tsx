import { Alert, Stack } from '@mantine/core'
import { IconAlertTriangle } from '@tabler/icons-react'
import { useTranslation } from 'react-i18next'
import type { RootNoticesProps } from './types'

export function RootNotices({ roots }: RootNoticesProps) {
  const { t, i18n } = useTranslation('mods')
  const { t: tErrors } = useTranslation('errors')
  const caseVariants = roots.filter((root) => root.caseVariant).map((root) => root.name)
  const unreadable = roots.flatMap((root) =>
    root.unreadable ? [{ name: root.name, problem: root.unreadable }] : [],
  )
  if (caseVariants.length === 0 && unreadable.length === 0) return null

  return (
    <Stack gap="xs">
      {caseVariants.length > 0 && (
        <Alert color="yellow" icon={<IconAlertTriangle size={20} stroke={1.8} />}>
          {t(($) => $.notices.caseVariantRoots, {
            names: new Intl.ListFormat(i18n.language, { type: 'conjunction' }).format(caseVariants),
          })}
        </Alert>
      )}
      {unreadable.map(({ name, problem }) => (
        <Alert key={name} color="yellow" icon={<IconAlertTriangle size={20} stroke={1.8} />}>
          {t(($) => $.notices.unreadableRoot, {
            root: name,
            reason: tErrors(($) => $.ioClasses[problem.class]),
          })}
        </Alert>
      ))}
    </Stack>
  )
}
