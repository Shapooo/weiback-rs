import React, { useState, useEffect, useMemo } from 'react';
import {
    Autocomplete,
    TextField,
    InputAdornment,
    Select,
    MenuItem,
    CircularProgress,
    Box,
    Typography,
    SelectChangeEvent
} from '@mui/material';
import { User } from '../types';
import { searchIdByUsernamePrefix } from '../lib/api';

type SearchMode = 'username' | 'id';

interface UserSelectorProps {
    value: User | string | null;
    onChange: (value: User | string | null) => void;
    label?: string;
    placeholderUsername?: string;
    placeholderId?: string;
    fullWidth?: boolean;
}

const UserSelector: React.FC<UserSelectorProps> = ({
    value,
    onChange,
    label = "用户",
    placeholderUsername = "输入用户名搜索...",
    placeholderId = "输入用户ID...",
    fullWidth = true,
}) => {
    const [searchMode, setSearchMode] = useState<SearchMode>('username');
    const [inputValue, setInputValue] = useState('');
    const [options, setOptions] = useState<User[]>([]);
    const [loading, setLoading] = useState(false);

    // When the controlled value changes, update the internal input value
    useEffect(() => {
        if (value) {
            if (typeof value === 'object') {
                setInputValue(value.screen_name);
            } else {
                setInputValue(value);
            }
        } else {
            setInputValue('');
        }
    }, [value]);


    const fetchUsers = useMemo(() => {
        const debouncedFetch = (prefix: string) => {
            if (prefix.trim() === '') {
                setOptions([]);
                return;
            }
            setLoading(true);
            searchIdByUsernamePrefix(prefix)
                .then((users) => {
                    setOptions(users);
                })
                .catch((err) => {
                    console.error("Failed to search users:", err);
                    setOptions([]);
                })
                .finally(() => {
                    setLoading(false);
                });
        };

        let timeoutId: number;
        return (prefix: string) => {
            clearTimeout(timeoutId);
            timeoutId = setTimeout(() => debouncedFetch(prefix), 300);
        };
    }, []);

    useEffect(() => {
        if (searchMode === 'username' && inputValue) {
            fetchUsers(inputValue);
        } else {
            setOptions([]);
        }
    }, [inputValue, searchMode, fetchUsers]);

    const handleModeChange = (event: SelectChangeEvent<SearchMode>) => { // Used SelectChangeEvent
        const newMode = event.target.value as SearchMode;
        setSearchMode(newMode);
        onChange(null); // Clear value when changing mode
    };

    return (
        <Autocomplete
            fullWidth={fullWidth}
            freeSolo={searchMode === 'id'}
            value={value}
            onChange={(_event, newValue) => {
                onChange(newValue);
            }}
            inputValue={inputValue}
            onInputChange={(_event, newInputValue) => {
                setInputValue(newInputValue);
            }}
            options={options}
            loading={loading}
            getOptionLabel={(option) => {
                if (typeof option === 'string') {
                    return option; // For freeSolo ID input
                }
                return option.screen_name;
            }}
            renderOption={(props, option) => (
                <Box component="li" {...props}>
                    <Typography variant="body1">{option.screen_name} <Typography variant="caption" color="textSecondary">({option.id})</Typography></Typography>
                </Box>
            )}
            renderInput={(params) => {
                const { InputProps, ...otherParams } = params; // Extract InputProps from params

                return (
                    <TextField
                        {...otherParams} // Spread the rest of the TextField params
                        label={label}
                        type={searchMode === 'id' ? 'number' : 'text'}
                        placeholder={searchMode === 'username' ? placeholderUsername : placeholderId}
                        slotProps={{
                            input: { // These are props for the InputBase component, which is the 'input' slot
                                ...InputProps, // Spread the original InputProps from Autocomplete here
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
                                        {InputProps.endAdornment} {/* Keep Autocomplete's endAdornment if any */}
                                    </React.Fragment>
                                ),
                            },
                        }}
                    />
                );
            }}
        />
    );
};

export default UserSelector;
