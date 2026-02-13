import { create } from 'zustand';
import { User } from '../types';
import { loginState } from '../lib/api';

interface AuthState {
    userInfo: User | null;
    isLoggedIn: boolean;
    isAuthLoading: boolean;
    checkLoginState: () => Promise<void>;
    login: (user: User) => void;
    logout: () => void;
}

export const useAuthStore = create<AuthState>((set) => ({
    userInfo: null,
    isLoggedIn: false,
    isAuthLoading: true, // Start with loading state true

    checkLoginState: async () => {
        try {
            set({ isAuthLoading: true });
            const user = await loginState();
            if (user) {
                set({ userInfo: user, isLoggedIn: true });
            } else {
                set({ userInfo: null, isLoggedIn: false });
            }
        } catch (error) {
            console.error("Failed to check login state:", error);
            set({ userInfo: null, isLoggedIn: false });
        } finally {
            set({ isAuthLoading: false });
        }
    },

    login: (user: User) => {
        set({ userInfo: user, isLoggedIn: true });
    },

    logout: () => {
        // Here you might want to call a backend logout function in the future
        // For now, it just clears the frontend state.
        set({ userInfo: null, isLoggedIn: false });
    },
}));
