import React, { useState, useEffect, useCallback, useRef } from 'react'
import {
  Autocomplete,
  TextField,
  InputAdornment,
  Select,
  MenuItem,
  CircularProgress,
  Box,
  Typography,
  SelectChangeEvent,
} from '@mui/material'
import { User } from '../types'
import { searchIdByUsernamePrefix } from '../lib/api'

type SearchMode = 'username' | 'id'

interface UserSelectorProps {
  value: User | string | null
  onChange: (value: User | string | null) => void
  label?: string
  placeholderUsername?: string
  placeholderId?: string
  fullWidth?: boolean
}

const UserSelector: React.FC<UserSelectorProps> = ({
  value,
  onChange,
  label = '用户',
  placeholderUsername = '输入用户名搜索...',
  placeholderId = '输入用户ID...',
  fullWidth = true,
}) => {
  const [searchMode, setSearchMode] = useState<SearchMode>('username')
  const [inputValue, setInputValue] = useState('')
  const [options, setOptions] = useState<User[]>([])
  const [loading, setLoading] = useState(false)
  const timeoutIdRef = useRef<number | undefined>(undefined)

  // When the controlled value changes, update the internal input value
  useEffect(() => {
    if (value) {
      if (typeof value === 'object') {
        setInputValue(value.screen_name)
      } else {
        setInputValue(value)
      }
    } else {
      setInputValue('')
    }
  }, [value])

  const fetchUsers = useCallback((prefix: string) => {
    if (timeoutIdRef.current) {
      clearTimeout(timeoutIdRef.current)
    }

    if (prefix.trim() === '') {
      setOptions([])
      return
    }

    timeoutIdRef.current = setTimeout(async () => {
      setLoading(true)
      try {
        const users = await searchIdByUsernamePrefix(prefix)
        setOptions(users)
      } catch (err) {
        console.error('Failed to search users:', err)
        setOptions([])
      } finally {
        setLoading(false)
      }
    }, 300)
  }, [])

  useEffect(() => {
    return () => {
      if (timeoutIdRef.current) clearTimeout(timeoutIdRef.current)
    }
  }, [])

  useEffect(() => {
    if (searchMode === 'username' && inputValue) {
      fetchUsers(inputValue)
    } else {
      setOptions([])
    }
  }, [inputValue, searchMode, fetchUsers])

  const handleModeChange = (event: SelectChangeEvent<SearchMode>) => {
    // Used SelectChangeEvent
    const newMode = event.target.value as SearchMode
    setSearchMode(newMode)
    onChange(null) // Clear value when changing mode
  }

  return (
    <Autocomplete
      fullWidth={fullWidth}
      freeSolo={searchMode === 'id'}
      value={value}
      onChange={(_event, newValue) => {
        onChange(newValue)
      }}
      inputValue={inputValue}
      onInputChange={(_event, newInputValue) => {
        setInputValue(newInputValue)
      }}
      options={options}
      loading={loading}
      getOptionLabel={option => {
        if (typeof option === 'string') {
          return option // For freeSolo ID input
        }
        return option.screen_name
      }}
      renderOption={(props, option) => {
        const { key, ...rest } = props
        return (
          <Box component="li" key={key} {...rest}>
            <Typography variant="body1">
              {option.screen_name}{' '}
              <Typography variant="caption" color="textSecondary">
                ({option.id})
              </Typography>
            </Typography>
          </Box>
        )
      }}
      renderInput={params => {
        const { id, disabled, fullWidth, size } = params
        const { ref: inputRef, className, onMouseDown } = params.slotProps.input
        return (
          <TextField
            id={id}
            disabled={disabled}
            fullWidth={fullWidth}
            size={size}
            label={label}
            type={searchMode === 'id' ? 'number' : 'text'}
            placeholder={searchMode === 'username' ? placeholderUsername : placeholderId}
            slotProps={{
              htmlInput: params.slotProps.htmlInput,
              input: {
                ref: inputRef,
                className,
                onMouseDown,
                onBlur: (_e: React.FocusEvent<HTMLInputElement>) => {
                  if (searchMode === 'id' && inputValue.trim()) {
                    onChange(inputValue.trim())
                  }
                },
                startAdornment: (
                  <InputAdornment position="start">
                    <Select
                      variant="standard"
                      disableUnderline
                      value={searchMode}
                      onChange={handleModeChange}
                    >
                      <MenuItem value="username">用户名</MenuItem>
                      <MenuItem value="id">ID</MenuItem>
                    </Select>
                  </InputAdornment>
                ),
                endAdornment: (
                  <React.Fragment>
                    {loading ? <CircularProgress color="inherit" size={20} /> : null}
                  </React.Fragment>
                ),
              },
            }}
          />
        )
      }}
    />
  )
}

export default UserSelector
