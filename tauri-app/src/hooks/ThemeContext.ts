import { createContext, useContext } from 'react'

export const ThemeContext = createContext({
  toggleColorMode: () => {},
})

export const useThemeContext = () => useContext(ThemeContext)
