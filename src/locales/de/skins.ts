import type { skins as en } from '../en/skins'
import type { LocaleShape } from '../types'

export const skins: LocaleShape<typeof en> = {
  title: 'Skin-Sets',
  unavailable:
    'Keine TEKKEN-8-Skin-Speicherstände gefunden. Starte das Spiel einmal und komm dann zurück.',
  empty:
    'Ein Set bewahrt alle deine gespeicherten Skins auf, so hast du mehr als das Spiel Plätze hat.',
  saveCurrent: 'Aktuelle Skins als Set speichern',
  saveFirst: 'Meine Skins speichern',
  inGame: 'Im Spiel',
  switch: 'Wechseln',
  switchTo: 'Zu {{name}} wechseln',
  switched: 'Skins zu {{name}} gewechselt. Deine bisherigen Skins bleiben in ihrem Set.',
  interrupted:
    'Ein Wechsel zu {{name}} wurde nicht beendet. Bis dahin können deine Skins gemischt sein.',
  finish: 'Wechsel beenden',
  readOnly: 'Diese Liste stammt von einem neueren RemMods und lässt sich hier nicht ändern.',
  newSet: {
    title: 'Aktuelle Skins als Set speichern',
    hint: 'Kopiert deine gespeicherten Skins in ein neues Set, das dann als das Set im Spiel gilt. Schließe vorher das Spiel. Kampfgeld, Gegenstände und Replays gehören nicht zu einem Set.',
    name: 'Name',
    create: 'Set speichern',
    cancel: 'Abbrechen',
  },
  problem: {
    taken: 'Ein Set mit diesem Namen gibt es schon.',
    invalid: 'Verwende einen Namen mit bis zu 100 Zeichen.',
  },
  dialog: {
    name: 'Name',
    notes: 'Notizen',
    notesHint: 'Was steckt in diesem Set? Zum Beispiel: Reina Platz 3 ist das Maid-Outfit.',
    save: 'Änderungen speichern',
    size: 'Gespeicherte Kopie: {{size}}',
    delete: 'Set löschen',
    deleteActive: 'Dieses Set ist im Spiel. Wechsle zuerst zu einem anderen.',
    putBack: 'Wieder ins Spiel holen',
  },
  putBack: {
    title: '„{{name}}“ zurückholen?',
    body: 'Die Skins im Spiel werden durch das ersetzt, was dieses Set bei deinem letzten Wechsel gespeichert hat. Nutze das, wenn sie kaputt sind, zum Beispiel nach einer Neuinstallation. Was das Spiel jetzt hat, bleibt in den automatischen Kopien.',
    confirm: 'Zurückholen',
    cancel: 'So lassen',
    done: '{{name}} ist wieder im Spiel.',
  },
  copies: {
    title: 'Automatische Kopien',
    hint: 'RemMods legt sie bei jedem Wechsel selbst an. Um Skins zurückzuholen, behalte eine Kopie als Set und wechsle dann zu diesem Set.',
    empty: 'Noch keine Kopien. Die erste entsteht bei deinem ersten Wechsel.',
    before: 'Skins von vor einem Wechsel',
    earlier: 'Frühere Kopie von {{name}}',
    keep: 'Als Set behalten',
    loadError: 'Die Kopien konnten nicht gelesen werden.',
  },
  keepCopy: {
    title: 'Diese Kopie als Set behalten',
    hint: 'Macht aus dieser Kopie ein neues Set. Die Skins im Spiel bleiben, wie sie sind, bis du zu dem Set wechselst.',
  },
  shots: {
    title: 'Screenshots',
    hint: 'RemMods kann nicht in die Speicherstände schauen, darum zeigt ein Screenshot jeder Platz-Seite, was ein Set enthält. Drücke im Spiel F12 und füge sie dann hier hinzu oder lege sie auf dem Fenster ab.',
    add: 'Screenshots hinzufügen…',
    pickTitle: 'Screenshots für dieses Set wählen',
    dropLabel: 'Ablegen, um sie zu {{name}} hinzuzufügen',
    alt: 'Screenshot einer Platz-Seite',
    remove: 'Screenshot entfernen',
    zoomIn: 'Größer zeigen',
    zoomOut: 'Kleiner zeigen',
  },
  delete: {
    title: '„{{name}}“ löschen?',
    body: 'Die Skins in diesem Set sind dann endgültig weg, außer sie sind auch im Spiel oder in einem anderen Set.',
    confirm: 'Set löschen',
    cancel: 'Behalten',
  },
}
