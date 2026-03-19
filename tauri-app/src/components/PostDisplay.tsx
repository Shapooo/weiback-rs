import React, { useState } from 'react'
import { useSnackbar } from 'notistack'
import {
  Avatar,
  Box,
  Typography,
  Card,
  CardContent,
  CardHeader,
  IconButton,
  Stack,
  CircularProgress,
  Tooltip,
  Dialog,
  DialogActions,
  DialogContent,
  DialogContentText,
  DialogTitle,
  Button,
  Link,
} from '@mui/material'
import OpenInNewIcon from '@mui/icons-material/OpenInNew'
import DeleteIcon from '@mui/icons-material/Delete'
import SyncIcon from '@mui/icons-material/Sync'
import BrokenImageIcon from '@mui/icons-material/BrokenImage'
import LinkIcon from '@mui/icons-material/Link'
import ImageIcon from '@mui/icons-material/Image'
import PlayArrowIcon from '@mui/icons-material/PlayArrow'
import ArticleIcon from '@mui/icons-material/Article'
import { openUrl } from '@tauri-apps/plugin-opener'
import { avatarCache, attachedCache } from '../cache'
import { useImageLoader } from '../hooks/useImageLoader'
import Emoji from './Emoji'
import { PostInfo, UrlStructItem, AttachedImage as AttachedImageData } from '../types'
import { deletePost, rebackupPost } from '../lib/api'

// --- Type Definitions are now in ../types.ts ---

interface AvatarImageProps {
  avatarId: string | null
}

const AvatarImage = React.memo(function AvatarImage({ avatarId }: AvatarImageProps) {
  const { imageUrl } = useImageLoader(avatarId, avatarCache)

  return <Avatar src={imageUrl} />
})

const THUMBNAIL_SIZE = 70 // Define a consistent size for thumbnails

interface AttachedImageProps {
  image: AttachedImageData
  size: number
  onClick: (image: AttachedImageData) => void
}

const AttachedImage = React.memo(function AttachedImage({
  image,
  size,
  onClick,
}: AttachedImageProps) {
  const imageId = image.data.id
  const { status, imageUrl } = useImageLoader(imageId, attachedCache)

  const commonSx = {
    width: size,
    height: size,
    borderRadius: 1,
    flexShrink: 0,
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
  }

  if (status === 'loading') {
    return (
      <Box sx={commonSx}>
        <CircularProgress size={20} />
      </Box>
    )
  }

  if (status === 'error') {
    return (
      <Box sx={{ ...commonSx, bgcolor: 'grey.200' }}>
        <BrokenImageIcon color="action" />
      </Box>
    )
  }

  // status === 'loaded'
  return (
    <Box
      onClick={e => {
        e.stopPropagation()
        if (image.type === 'video_cover' && image.data.video_url) {
          openUrl(image.data.video_url).catch(err =>
            console.error('Failed to open video URL:', err)
          )
        } else if (image.type === 'article_cover' && image.data.url) {
          openUrl(image.data.url).catch(err => console.error('Failed to open article URL:', err))
        } else {
          onClick(image)
        }
      }}
      sx={{
        ...commonSx,
        backgroundImage: `url(${imageUrl})`,
        backgroundSize: 'cover',
        backgroundPosition: 'center',
        cursor: 'pointer',
        position: 'relative',
      }}
    >
      {image.type === 'livephoto' && (
        <Box
          sx={{
            position: 'absolute',
            top: 4,
            left: 4,
            bgcolor: 'rgba(0, 0, 0, 0.5)',
            color: 'white',
            fontSize: '9px',
            px: 0.5,
            borderRadius: 0.5,
            fontWeight: 'bold',
            pointerEvents: 'none',
          }}
        >
          LIVE
        </Box>
      )}
      {image.type === 'video_cover' && (
        <Box
          sx={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            width: '100%',
            height: '100%',
            bgcolor: 'rgba(0, 0, 0, 0.2)',
          }}
        >
          <PlayArrowIcon sx={{ color: 'white', opacity: 0.8, fontSize: size / 2 }} />
        </Box>
      )}
    </Box>
  )
})

interface ArticleCoverProps {
  image: Extract<AttachedImageData, { type: 'article_cover' }>
}

const ArticleCover = React.memo(function ArticleCover({ image }: ArticleCoverProps) {
  const imageId = image.data.id
  const title = image.data.title
  const url = image.data.url
  const { status, imageUrl } = useImageLoader(imageId, attachedCache)

  if (status === 'loading') {
    return (
      <Box
        sx={{
          width: '100%',
          height: 200,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          bgcolor: 'grey.100',
          borderRadius: 1,
        }}
      >
        <CircularProgress />
      </Box>
    )
  }

  if (status === 'error') {
    return (
      <Box
        sx={{
          width: '100%',
          height: 80,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          bgcolor: 'grey.200',
          borderRadius: 1,
          gap: 1,
        }}
      >
        <BrokenImageIcon color="action" />
        <Typography variant="body2" color="text.secondary">
          文章封面已失效
        </Typography>
      </Box>
    )
  }

  return (
    <Box
      onClick={e => {
        e.stopPropagation()
        openUrl(url).catch(err => console.error('Failed to open article URL:', err))
      }}
      sx={{
        width: '100%',
        cursor: 'pointer',
        position: 'relative',
        borderRadius: 1,
        overflow: 'hidden',
        '&:hover': {
          opacity: 0.9,
        },
      }}
    >
      <Box
        sx={{
          width: '100%',
          paddingBottom: '56.25%',
          position: 'relative',
          backgroundImage: `url(${imageUrl})`,
          backgroundSize: 'cover',
          backgroundPosition: 'center',
        }}
      >
        <Box
          sx={{
            position: 'absolute',
            bottom: 0,
            left: 0,
            right: 0,
            background: 'linear-gradient(to top, rgba(0,0,0,0.8) 0%, rgba(0,0,0,0) 100%)',
            color: 'white',
            p: 2,
            display: 'flex',
            alignItems: 'center',
            gap: 1,
          }}
        >
          <ArticleIcon sx={{ fontSize: 20, opacity: 0.9 }} />
          <Typography variant="body2" sx={{ fontWeight: 500, flex: 1 }}>
            {title}
          </Typography>
          <OpenInNewIcon sx={{ fontSize: 18, opacity: 0.7 }} />
        </Box>
      </Box>
    </Box>
  )
})

interface AttachedImagesProps {
  attachedImages: AttachedImageData[]
  onImageClick: (image: AttachedImageData) => void
  maxImages?: number
}

const AttachedImages = React.memo(function AttachedImages({
  attachedImages,
  onImageClick,
  maxImages,
}: AttachedImagesProps) {
  if (!attachedImages || attachedImages.length === 0) {
    return null
  }

  const articleCover = attachedImages.find(img => img.type === 'article_cover')
  if (articleCover) {
    return (
      <Box sx={{ mt: 1 }}>
        <ArticleCover image={articleCover} />
      </Box>
    )
  }

  const displayedImages =
    maxImages !== undefined ? attachedImages.slice(0, maxImages) : attachedImages
  const remainingCount = attachedImages.length - displayedImages.length

  return (
    <Stack direction="row" spacing={1} sx={{ mt: 1, flexWrap: 'wrap' }}>
      {displayedImages.map((img, idx) => (
        <AttachedImage
          key={`${img.data.id}-${idx}`}
          image={img}
          size={THUMBNAIL_SIZE}
          onClick={onImageClick}
        />
      ))}
      {remainingCount > 0 && (
        <Box
          sx={{
            width: THUMBNAIL_SIZE,
            height: THUMBNAIL_SIZE,
            backgroundColor: 'rgba(0, 0, 0, 0.5)',
            borderRadius: 1,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            flexShrink: 0,
          }}
        >
          <Typography variant="body2" color="white">
            +{remainingCount}
          </Typography>
        </Box>
      )}
    </Stack>
  )
})

interface ProcessedTextProps {
  text: string
  emoji_map: Record<string, string>
  url_struct: UrlStructItem[] | null
  maxLines?: number
  inline_map?: Record<string, string>
  onImageClick?: (image: AttachedImageData) => void
}

const URL_REGEX = /https?:\/\/[a-zA-Z0-9$%&~_#./\-:=,?]{5,280}/g
const AT_REGEX = /@[\u4e00-\u9fa5\uE7C7-\uE7F3\w_\-·]+/gu
const TOPIC_REGEX = /#[^#]+#/g
const EMAIL_REGEX = /[A-Za-z0-9]+([_.][A-Za-z0-9]+)*@([A-Za-z0-9-]+\.)+[A-Za-z]{2,6}/g
const EMOJI_REGEX = /\[.*?\]/g

const COMBINED_REGEX = new RegExp(
  `(${URL_REGEX.source})|(${AT_REGEX.source})|(${TOPIC_REGEX.source})|(${EMOJI_REGEX.source})`,
  'gu'
)

const ProcessedText = React.memo(function ProcessedText({
  text,
  emoji_map,
  url_struct,
  maxLines,
  inline_map,
  onImageClick,
}: ProcessedTextProps) {
  const inPreviewMode = maxLines !== undefined
  const processedText = inPreviewMode ? text.replace(/\n/g, ' ') : text

  const urlMap = new Map<string, UrlStructItem>()
  if (url_struct) {
    for (const item of url_struct) {
      urlMap.set(item.short_url, item)
    }
  }

  const emailSuffixes = new Set<string>()
  for (const match of processedText.matchAll(EMAIL_REGEX)) {
    const email = match[0]
    const atMatch = email.match(AT_REGEX)
    if (atMatch) {
      emailSuffixes.add(atMatch[0])
    }
  }

  const nodes: React.ReactNode[] = []
  let lastIndex = 0

  for (const match of processedText.matchAll(COMBINED_REGEX)) {
    const fullMatch = match[0]
    const url = match[1]
    const at = match[2]
    const topic = match[3]
    const emoji = match[4]
    const matchIndex = match.index!

    if (matchIndex > lastIndex) {
      nodes.push(processedText.substring(lastIndex, matchIndex))
    }

    if (url) {
      const inlinePicId = inline_map?.[url]
      const urlData = urlMap.get(url)
      if (inlinePicId && onImageClick) {
        nodes.push(
          <Link
            key={lastIndex}
            component="button"
            variant="body2"
            onClick={e => {
              e.stopPropagation()
              onImageClick({ type: 'normal', data: { id: inlinePicId } })
            }}
            sx={{
              verticalAlign: 'middle',
              display: 'inline-flex',
              alignItems: 'center',
              cursor: 'pointer',
              border: 'none',
              background: 'none',
              p: 0,
            }}
          >
            <ImageIcon sx={{ marginRight: '0.2em', width: '16px', height: '16px' }} />
            查看图片
          </Link>
        )
      } else if (urlData) {
        const longUrl = urlData.long_url || url
        const title = urlData.url_title || '网页链接'
        nodes.push(
          <Link key={lastIndex} href={longUrl} target="_blank" rel="noopener noreferrer">
            <LinkIcon
              sx={{
                marginRight: '0.2em',
                verticalAlign: 'middle',
                width: '16px',
                height: '16px',
              }}
            />
            {title}
          </Link>
        )
      } else {
        nodes.push(
          <Link key={lastIndex} href={url} target="_blank" rel="noopener noreferrer">
            {url}
          </Link>
        )
      }
    } else if (at) {
      if (emailSuffixes.has(at)) {
        nodes.push(at)
      } else {
        const username = at.substring(1)
        nodes.push(
          <Link
            key={lastIndex}
            href={`https://weibo.com/n/${username}`}
            target="_blank"
            rel="noopener noreferrer"
          >
            {at}
          </Link>
        )
      }
    } else if (topic) {
      nodes.push(
        <Link
          key={lastIndex}
          href={`https://s.weibo.com/weibo?q=${encodeURIComponent(topic)}`}
          target="_blank"
          rel="noopener noreferrer"
        >
          {topic}
        </Link>
      )
    } else if (emoji) {
      const imageId = emoji_map[emoji]
      if (imageId) {
        nodes.push(<Emoji key={lastIndex} imageId={imageId} emojiText={emoji} />)
      } else {
        nodes.push(emoji)
      }
    }

    lastIndex = matchIndex + fullMatch.length
  }

  if (lastIndex < processedText.length) {
    nodes.push(processedText.substring(lastIndex))
  }

  const content = nodes.map((node, index) => <React.Fragment key={index}>{node}</React.Fragment>)

  const previewStyles = inPreviewMode
    ? {
        display: '-webkit-box',
        WebkitLineClamp: maxLines,
        WebkitBoxOrient: 'vertical',
        overflow: 'hidden',
        textOverflow: 'ellipsis',
      }
    : {}

  return (
    <Typography
      variant="body2"
      component="div"
      sx={{
        ...previewStyles,
        wordBreak: 'break-word',
        lineHeight: '1.8',
        ...(inPreviewMode ? {} : { whiteSpace: 'pre-wrap' }),
      }}
    >
      {content}
    </Typography>
  )
})

interface PostDisplayProps {
  postInfo: PostInfo
  onImageClick: (image: AttachedImageData) => void
  maxAttachedImages?: number // Prop to limit displayed attached images
  onClick?: (postInfo: PostInfo) => void
  maxLines?: number // New prop for text truncation
  onPostDeleted?: () => void
}

const PostDisplay: React.FC<PostDisplayProps> = ({
  postInfo,
  onImageClick,
  maxAttachedImages,
  onClick,
  maxLines,
  onPostDeleted,
}) => {
  const { enqueueSnackbar } = useSnackbar()
  const [dialogOpen, setDialogOpen] = useState(false)

  const handleDeleteClick = (e: React.MouseEvent) => {
    e.stopPropagation()
    setDialogOpen(true)
  }

  const handleDialogClose = () => {
    setDialogOpen(false)
  }

  const handleDeleteConfirm = async (e: React.MouseEvent) => {
    e.stopPropagation()
    try {
      await deletePost(postInfo.post.id)
      enqueueSnackbar('帖子已删除', { variant: 'success' })
      onPostDeleted?.()
    } catch (error) {
      console.error('Failed to delete post:', error)
      enqueueSnackbar(`删除失败: ${error}`, { variant: 'error' })
    } finally {
      setDialogOpen(false)
    }
  }

  const handleRebackupClick = async (e: React.MouseEvent) => {
    e.stopPropagation()
    try {
      await rebackupPost(postInfo.post.id.toString())
      enqueueSnackbar('已加入重新备份队列', { variant: 'info' })
    } catch (error) {
      console.error('Failed to re-backup post:', error)
      enqueueSnackbar(`重新备份失败: ${error}`, { variant: 'error' })
    }
  }

  return (
    <>
      <Card onClick={() => onClick?.(postInfo)} sx={{ cursor: onClick ? 'pointer' : 'default' }}>
        <CardHeader
          avatar={<AvatarImage avatarId={postInfo.avatar_id} />}
          title={postInfo.post.user?.screen_name || '未知用户'}
          subheader={new Date(postInfo.post.created_at).toLocaleString()}
          action={
            <Stack direction="row" alignItems="center">
              {postInfo.post.id ? (
                <Tooltip title="重新备份" enterDelay={500} arrow>
                  <IconButton aria-label="re-backup post" onClick={handleRebackupClick}>
                    <SyncIcon />
                  </IconButton>
                </Tooltip>
              ) : null}
              {postInfo.post.user?.id && postInfo.post.id ? (
                <Tooltip
                  title={`https://weibo.com/${postInfo.post.user.id}/${postInfo.post.id}`}
                  enterDelay={500}
                  arrow
                >
                  <IconButton
                    aria-label="open original post"
                    onClick={e => {
                      e.stopPropagation()
                      const url = `https://weibo.com/${postInfo.post.user!.id}/${postInfo.post.id}`
                      openUrl(url).catch(e => console.error('Failed to open URL:', e))
                    }}
                  >
                    <OpenInNewIcon />
                  </IconButton>
                </Tooltip>
              ) : null}
              {postInfo.post.id ? (
                <Tooltip title="删除" enterDelay={500} arrow>
                  <IconButton aria-label="delete post" onClick={handleDeleteClick}>
                    <DeleteIcon />
                  </IconButton>
                </Tooltip>
              ) : null}
            </Stack>
          }
        />
        <CardContent>
          <ProcessedText
            text={postInfo.post.text}
            emoji_map={postInfo.emoji_map}
            url_struct={postInfo.post.url_struct}
            maxLines={maxLines}
            inline_map={postInfo.inline_map}
            onImageClick={onImageClick}
          />
          {postInfo.post.retweeted_status && (
            <Box sx={{ mt: 2, p: 2, backgroundColor: 'grey.100', borderRadius: 1 }}>
              <Typography variant="subtitle2" color="text.secondary">
                @{postInfo.post.retweeted_status.user?.screen_name || '未知用户'}
              </Typography>
              <ProcessedText
                text={postInfo.post.retweeted_status.text}
                emoji_map={postInfo.emoji_map}
                url_struct={postInfo.post.retweeted_status.url_struct}
                maxLines={maxLines}
              />
            </Box>
          )}
          <AttachedImages
            attachedImages={postInfo.standalone_pics}
            onImageClick={onImageClick}
            maxImages={maxAttachedImages}
          />
        </CardContent>
      </Card>
      <Dialog
        open={dialogOpen}
        onClose={handleDialogClose}
        onClick={e => e.stopPropagation()}
        aria-labelledby="alert-dialog-title"
        aria-describedby="alert-dialog-description"
      >
        <DialogTitle id="alert-dialog-title">{'确认删除'}</DialogTitle>
        <DialogContent>
          <DialogContentText id="alert-dialog-description">
            删除会删除该post及该post的所有转发以及图片等资源，且无法撤回。你确定要删除吗？
          </DialogContentText>
        </DialogContent>
        <DialogActions>
          <Button onClick={handleDialogClose}>取消</Button>
          <Button onClick={handleDeleteConfirm} color="error" autoFocus>
            确认
          </Button>
        </DialogActions>
      </Dialog>
    </>
  )
}

export default PostDisplay
