import { Injectable } from '@angular/core';
import { MatSnackBar } from '@angular/material/snack-bar';

export type NotificationType = 'success' | 'error' | 'info' | 'warning';

/**
 * Service for displaying toast notifications in the application.
 */
@Injectable({
  providedIn: 'root'
})
export class NotificationService {
  constructor(private snackBar: MatSnackBar) {}

  /**
   * Shows a notification message
   * 
   * @param message The message to display
   * @param type The type of notification (success, error, info, warning)
   * @param duration Duration in milliseconds (default: 5000ms)
   */
  show(message: string, type: NotificationType = 'info', duration = 5000): void {
    const panelClass = `notification-${type}`;
    
    this.snackBar.open(message, 'Close', {
      duration,
      horizontalPosition: 'right',
      verticalPosition: 'top',
      panelClass: [panelClass]
    });
  }

  /**
   * Shows a success notification
   * 
   * @param message The message to display
   * @param duration Duration in milliseconds (default: 5000ms)
   */
  success(message: string, duration = 5000): void {
    this.show(message, 'success', duration);
  }

  /**
   * Shows an error notification
   * 
   * @param message The message to display
   * @param duration Duration in milliseconds (default: 8000ms)
   */
  error(message: string, duration = 8000): void {
    this.show(message, 'error', duration);
  }

  /**
   * Shows an info notification
   * 
   * @param message The message to display
   * @param duration Duration in milliseconds (default: 5000ms)
   */
  info(message: string, duration = 5000): void {
    this.show(message, 'info', duration);
  }

  /**
   * Shows a warning notification
   * 
   * @param message The message to display
   * @param duration Duration in milliseconds (default: 6000ms)
   */
  warning(message: string, duration = 6000): void {
    this.show(message, 'warning', duration);
  }
}
