import { Injectable } from '@angular/core';
import { MatDialog } from '@angular/material/dialog';
import { DialogComponent, DialogData } from '../../shared/components/dialog/dialog.component';
import { Observable } from 'rxjs';

/**
 * Service for displaying dialog boxes throughout the application
 */
@Injectable({
  providedIn: 'root'
})
export class DialogService {
  constructor(private dialog: MatDialog) {}

  /**
   * Opens a confirmation dialog
   * 
   * @param title Dialog title
   * @param message Dialog message
   * @param confirmText Text for confirm button
   * @param cancelText Text for cancel button
   * @returns Observable that resolves with true when confirmed, false when cancelled
   */
  confirm(title: string, message: string, confirmText = 'Confirm', cancelText = 'Cancel'): Observable<boolean> {
    const dialogRef = this.dialog.open(DialogComponent, {
      data: {
        title,
        message,
        confirmText,
        cancelText,
        type: 'confirm'
      } as DialogData,
      width: '400px'
    });

    return dialogRef.afterClosed();
  }

  /**
   * Opens an alert dialog
   * 
   * @param title Dialog title
   * @param message Dialog message
   * @param buttonText Text for OK button
   * @returns Observable that resolves when the user closes the dialog
   */
  alert(title: string, message: string, buttonText = 'OK'): Observable<void> {
    const dialogRef = this.dialog.open(DialogComponent, {
      data: {
        title,
        message,
        confirmText: buttonText,
        hideCancel: true,
        type: 'info'
      } as DialogData,
      width: '400px'
    });

    return dialogRef.afterClosed();
  }

  /**
   * Opens an error dialog
   * 
   * @param title Dialog title
   * @param message Error message
   * @param buttonText Text for OK button
   * @returns Observable that resolves when the user closes the dialog
   */
  error(title: string, message: string, buttonText = 'OK'): Observable<void> {
    const dialogRef = this.dialog.open(DialogComponent, {
      data: {
        title,
        message,
        confirmText: buttonText,
        hideCancel: true,
        type: 'error'
      } as DialogData,
      width: '400px'
    });

    return dialogRef.afterClosed();
  }

  /**
   * Opens a warning dialog
   * 
   * @param title Dialog title
   * @param message Warning message
   * @param confirmText Text for confirm button
   * @param cancelText Text for cancel button
   * @returns Observable that resolves with true when confirmed, false when cancelled
   */
  warning(title: string, message: string, confirmText = 'Proceed', cancelText = 'Cancel'): Observable<boolean> {
    const dialogRef = this.dialog.open(DialogComponent, {
      data: {
        title,
        message,
        confirmText,
        cancelText,
        type: 'warning'
      } as DialogData,
      width: '400px'
    });

    return dialogRef.afterClosed();
  }
}
