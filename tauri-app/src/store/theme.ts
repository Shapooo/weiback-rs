import { ref, computed } from 'vue';
import { darkTheme, GlobalThemeOverrides } from 'naive-ui';

// Shared state for dark mode
export const isDarkMode = ref<boolean>(false);

// Computed theme object for Naive UI
export const theme = computed(() => (isDarkMode.value ? darkTheme : null));

// Function to toggle the theme
export function toggleDarkMode(value: boolean) {
  isDarkMode.value = value;
}

// Centralized and reactive theme overrides
export const themeOverrides = computed<GlobalThemeOverrides>(() => {
  const common = {
    bodyColor: isDarkMode.value ? '#1e1e1e' : '#ffffff',
  };

  const menu = {
    // Your existing light-mode styles
    itemTextColor: '#000000',
    itemTextColorHover: '#000000',
    itemTextColorActive: '#000000',
    itemIconColor: '#000000',
    itemIconColorHover: '#000000',
    itemIconColorActive: '#000000',
    itemColorHover: '#f5f5f5',
    itemColorHover: '#e6f7ff',
    itemColorActive: '#e6f7ff',
    itemColorActiveCollapsed: '#e6f7ff',
  };

  if (isDarkMode.value) {
    // Override styles for dark mode
    menu.itemTextColor = 'rgba(255, 255, 255, 0.82)';
    menu.itemTextColorHover = 'rgba(255, 255, 255, 0.9)';
    menu.itemTextColorActive = 'rgba(255, 255, 255, 1)';
    menu.itemIconColor = 'rgba(255, 255, 255, 0.82)';
    menu.itemIconColorHover = 'rgba(255, 255, 255, 0.9)';
    menu.itemIconColorActive = 'rgba(255, 255, 255, 1)';
    menu.itemColorHover = 'rgba(255, 255, 255, 0.09)';
    menu.itemColorActive = 'rgba(64, 158, 255, 0.24)';
    menu.itemColorActiveCollapsed = 'rgba(64, 158, 255, 0.24)';
  }

  return { Menu: menu, common };
});
