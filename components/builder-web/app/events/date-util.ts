import * as moment from 'moment';

export const dateFilters = [
    { label: 'Last 1 Week', type: 'days', interval: 7 },
    { label: 'Last 2 Weeks', type: 'days', interval: 14 },
    { label: 'Last 1 Month', type: 'months', interval: 1 },
    { label: 'Last 3 Months', type: 'months', interval: 3 },
    { label: 'Last 6 Months', type: 'months', interval: 6 },
    { label: 'Last 1 Year', type: 'years', interval: 1 }
];

export function getDateRange(filter: any) {
    switch (filter.type) {
        case 'days':
            return getRange('days', filter.interval);
        case 'months':
            return getRange('months', filter.interval);
        case 'years':
            return getRange('years', filter.interval);
        case 'custom':
            return getCustomRange(filter.startDate, filter.endDate);
        default:
            // Should not happen this
            return getRange('days', 7);
    }
}

function getRange(type, interval) {
    const today = new Date();

    const from_date = moment(today).subtract(interval, type).format('YYYY-MM-DD');
    const to_date = moment(today).format('YYYY-MM-DD');

    return {
        fromDate: from_date,
        toDate: to_date
    };
}

function getCustomRange(fromDate: Date, toDate: Date) {
    const from_date = moment(fromDate).format('YYYY-MM-DD');
    const to_date = moment(toDate).format('YYYY-MM-DD');

    return {
        fromDate: from_date,
        toDate: to_date
    };
}

// Given a Date object, returns date in YYYY-MM-DD format
export function toDateString(date: Date) {
    return moment(date).format('YYYY-MM-DD');
}

// Given a date in string (YYYY-MM-DD), returns a Date object
export function toDate(date: string) {
    return moment(date).toDate();
}
