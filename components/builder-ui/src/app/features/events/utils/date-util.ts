import { formatDate } from '@angular/common';

export interface DateFilter {
    label: string;
    type: string;
    interval?: number;
    startDate?: Date;
    endDate?: Date;
}

export const dateFilters: DateFilter[] = [
    { label: 'Last 1 Week', type: 'days', interval: 7 },
    { label: 'Last 2 Weeks', type: 'days', interval: 14 },
    { label: 'Last 1 Month', type: 'months', interval: 1 },
    { label: 'Last 3 Months', type: 'months', interval: 3 },
    { label: 'Last 6 Months', type: 'months', interval: 6 },
    { label: 'Last 1 Year', type: 'years', interval: 1 }
];

export function getDateRange(filter: DateFilter) {
    switch (filter.type) {
        case 'days':
            return getRange('days', filter.interval || 7);
        case 'months':
            return getRange('months', filter.interval || 1);
        case 'years':
            return getRange('years', filter.interval || 1);
        case 'custom':
            return getCustomRange(filter.startDate!, filter.endDate!);
        default:
            // Default to last 7 days
            return getRange('days', 7);
    }
}

function getRange(type: 'days' | 'months' | 'years', interval: number) {
    const today = new Date();
    const fromDate = new Date();

    switch (type) {
        case 'days':
            fromDate.setDate(fromDate.getDate() - interval);
            break;
        case 'months':
            fromDate.setMonth(fromDate.getMonth() - interval);
            break;
        case 'years':
            fromDate.setFullYear(fromDate.getFullYear() - interval);
            break;
    }

    return {
        fromDate: formatDate(fromDate, 'yyyy-MM-dd', 'en-US'),
        toDate: formatDate(today, 'yyyy-MM-dd', 'en-US')
    };
}

function getCustomRange(fromDate: Date, toDate: Date) {
    return {
        fromDate: formatDate(fromDate, 'yyyy-MM-dd', 'en-US'),
        toDate: formatDate(toDate, 'yyyy-MM-dd', 'en-US')
    };
}

// Given a Date object, returns date in YYYY-MM-DD format
export function toDateString(date: Date): string {
    return formatDate(date, 'yyyy-MM-dd', 'en-US');
}

// Given a date in string (YYYY-MM-DD), returns a Date object
export function toDate(dateStr: string): Date {
    return new Date(dateStr);
}
