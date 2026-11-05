import { Image } from '@mantine/core'
import { IconPhotoOff } from '@tabler/icons-react'
import { useItemPicture } from '../item-picture'
import classes from './item-picture.module.css'
import type { ItemPictureProps } from './types'

// Transparent pictures sit on a tile, so light and dark themes show them alike.
export function ItemPicture({ id, label, width, height }: ItemPictureProps) {
  const picture = useItemPicture(id)
  const box = { width, height }

  if (!picture.data) {
    return (
      <div className={classes.tile} style={box} role="img" aria-label={label}>
        <IconPhotoOff size={20} stroke={1.5} />
      </div>
    )
  }
  return (
    <div className={classes.tile} style={box}>
      <Image src={picture.data} alt={label} fit="contain" h="100%" draggable={false} />
    </div>
  )
}
