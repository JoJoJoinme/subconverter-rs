import { Locale, defaultLocale } from '@/i18n/config';

// Modified for static export compatibility
// 'use server' and cookies() are not supported in output: 'export'

export async function getUserLocale() {
    // For static export, we default to the configured default locale
    // In a real server environment, this would read from cookies
    return defaultLocale;
}

export async function setUserLocale(locale: Locale) {
    // No-op for static export
    // Client-side code should handle cookie setting if needed
    console.log('setUserLocale called with', locale);
}
