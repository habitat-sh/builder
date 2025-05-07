import { Pipe, PipeTransform } from '@angular/core';

/**
 * Truncates text to a specified length and adds an ellipsis if needed.
 * Usage: {{ longText | truncate:50 }} or {{ longText | truncate:50:'...' }}
 */
@Pipe({
  name: 'truncate',
  standalone: true
})
export class TruncatePipe implements PipeTransform {
  transform(value: string, limit = 25, trail = '...'): string {
    if (!value) {
      return '';
    }

    if (value.length <= limit) {
      return value;
    }

    return value.substring(0, limit) + trail;
  }
}
