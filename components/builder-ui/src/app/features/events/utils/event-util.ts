// A collection of utilities for the events components

// Return a job state's proper icon symbol
export function iconForJobState(state: string): string {
    return {
        canceled: 'cancel',
        cancelpending: 'sync',
        cancelprocessing: 'sync',
        cancelcomplete: 'cancel',
        complete: 'check_circle',
        success: 'check_circle',
        dispatching: 'sync',
        dispatched: 'sync',
        failed: 'error',
        failure: 'error',
        inprogress: 'sync',
        notstarted: 'schedule',
        pending: 'schedule',
        processing: 'sync',
        queued: 'schedule',
        rejected: 'error',
        skipped: 'block',
        demote: 'arrow_downward',
        promote: 'arrow_upward',
    }[state.toLowerCase()] || 'help';
}

// Translate a job state into a friendlier label
export function labelForJobState(state: string): string {
    return {
        canceled: 'Canceled',
        cancelpending: 'Canceling',
        cancelprocessing: 'Canceling',
        cancelcomplete: 'Canceled',
        complete: 'Complete',
        success: 'Complete',
        dispatching: 'Dispatching',
        dispatched: 'Dispatched',
        failed: 'Failed',
        failure: 'Failed',
        inprogress: 'In Progress',
        notstarted: 'Not Started',
        pending: 'Pending',
        processing: 'Processing',
        queued: 'Queued',
        rejected: 'Rejected',
        skipped: 'Skipped',
        demote: 'Demote',
        promote: 'Promote',
    }[state.toLowerCase()] || 'Unknown';
}

// Format a package identifier into a string
export function packageString(pkg: any = {}): string {
    return ['origin', 'name', 'version', 'release']
        .map(part => pkg[part])
        .filter(part => part).join('/');
}
