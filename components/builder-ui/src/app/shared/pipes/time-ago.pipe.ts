import { Pipe, PipeTransform } from '@angular/core';

/**
 * Formats a relative time (like "5 minutes ago", "2 days ago", etc.)
 * Usage: {{ timestamp | timeAgo }}
 */
@Pipe({
  name: 'timeAgo',
  standalone: true
})
export class TimeAgoPipe implements PipeTransform {
  transform(value: string | Date | number): string {
    if (!value) {
      return '';
    }

    const now = new Date();
    const date = value instanceof Date ? value : new Date(value);
    
    // Check for invalid date
    if (isNaN(date.getTime())) {
      return 'Invalid date';
    }

    const seconds = Math.floor((now.getTime() - date.getTime()) / 1000);
    
    // Less than a minute
    if (seconds < 60) {
      return 'just now';
    }
    
    // Less than an hour
    if (seconds < 3600) {
      const minutes = Math.floor(seconds / 60);
      return `${minutes} minute${minutes > 1 ? 's' : ''} ago`;
    }
    
    // Less than a day
    if (seconds < 86400) {
      const hours = Math.floor(seconds / 3600);
      return `${hours} hour${hours > 1 ? 's' : ''} ago`;
    }
    
    // Less than a month (30 days)
    if (seconds < 2592000) {
      const days = Math.floor(seconds / 86400);
      return `${days} day${days > 1 ? 's' : ''} ago`;
    }
    
    // Less than a year
    if (seconds < 31536000) {
      const months = Math.floor(seconds / 2592000);
      return `${months} month${months > 1 ? 's' : ''} ago`;
    }
    
    // Years
    const years = Math.floor(seconds / 31536000);
    return `${years} year${years > 1 ? 's' : ''} ago`;
  }
}
