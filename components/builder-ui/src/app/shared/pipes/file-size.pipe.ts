import { Pipe, PipeTransform } from '@angular/core';

/**
 * Formats file size in bytes to a human-readable format
 * Usage: {{ fileSize | fileSize }} or {{ fileSize | fileSize:2 }}
 */
@Pipe({
  name: 'fileSize',
  standalone: true
})
export class FileSizePipe implements PipeTransform {
  transform(bytes: number | null | undefined, precision = 2): string {
    if (bytes === null || bytes === undefined) {
      return '0 Bytes';
    }

    const units = ['Bytes', 'KB', 'MB', 'GB', 'TB', 'PB'];
    if (bytes === 0) {
      return '0 Bytes';
    }

    const i = Math.floor(Math.log(bytes) / Math.log(1024));
    const size = bytes / Math.pow(1024, i);
    return `${size.toFixed(precision)} ${units[i]}`;
  }
}
