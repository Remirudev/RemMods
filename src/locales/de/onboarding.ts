import type { onboarding as en } from '../en/onboarding'
import type { LocaleShape } from '../types'

export const onboarding: LocaleShape<typeof en> = {
  title: 'Wo ist TEKKEN 8?',
  description: 'RemMods braucht den Spielordner von TEKKEN 8. Vorerst liest es ihn nur.',
  missing: {
    title: 'Der gespeicherte Spielordner fehlt',
    body: '{{path}} enthält TEKKEN 8 nicht mehr. Wähle den Ordner erneut.',
  },
  found: { title: 'Über Steam gefunden', none: 'Steam hat keine TEKKEN-8-Installation gemeldet.' },
  choose: {
    button: 'Ordner wählen …',
    dialogTitle: 'TEKKEN-8-Ordner wählen',
    hint: 'Wähle den Ordner TEKKEN 8 oder einen Ordner darin, zum Beispiel Paks.',
    cancelled: 'Kein Ordner gewählt. Du kannst den Pfad auch eingeben.',
  },
  manual: {
    toggle: 'Pfad stattdessen eingeben',
    label: 'Pfad zum Ordner TEKKEN 8',
    placeholder: '…/steamapps/common/TEKKEN 8',
    submit: 'Diesen Pfad verwenden',
    notAbsolute: 'Gib den vollständigen Pfad zum Ordner ein.',
  },
  invalid: { title: 'Das ist kein TEKKEN-8-Ordner', body: '{{path}}: {{reason}}' },
  detectAgain: 'Erneut suchen',
}
