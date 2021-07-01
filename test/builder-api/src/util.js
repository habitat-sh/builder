function getLast1WeekDateRange() {
    // to_date is inclusive
    const toDate = new Date();

    // from_date is inclusive
    let fromDate = new Date();
    fromDate = new Date(fromDate.getFullYear(), fromDate.getMonth(), fromDate.getDate() - 7);

    const toDateStr = toDate.getFullYear() + '-' + (toDate.getMonth() + 1) + '-' + toDate.getDate();
    const fromDateStr = fromDate.getFullYear() + '-' + (fromDate.getMonth() + 1) + '-' + fromDate.getDate();

    return {
        fromDate: fromDateStr,
        toDate: toDateStr
    };
}

// Given a URI/URL append the last one week date range query parameters
function appendDateRange(url) {
    const dateRange = getLast1WeekDateRange();
    return `${url}?from_date=${dateRange.fromDate}&to_date=${dateRange.toDate}`;
}

module.exports.appendDateRange = appendDateRange;
