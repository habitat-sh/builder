import { Component, Inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { MatDialogModule, MatDialogRef, MAT_DIALOG_DATA } from '@angular/material/dialog';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';

/**
 * Data interface for the simple confirmation dialog
 */
export interface SimpleConfirmDialogData {
  title: string;
  message: string;
  confirmButton?: string;
  cancelButton?: string;
  hideCancel?: boolean;
  danger?: boolean;
}

/**
 * A reusable simple confirmation dialog component
 */
@Component({
  selector: 'app-simple-confirm-dialog',
  standalone: true,
  imports: [
    CommonModule,
    MatDialogModule,
    MatButtonModule,
    MatIconModule
  ],
  template: `
    <div class="confirm-dialog" [class.danger]="data.danger">
      <header class="confirm-header">
        <mat-icon>{{ data.danger ? 'warning' : 'info' }}</mat-icon>
        <h2>{{ data.title }}</h2>
      </header>
      <div class="confirm-content">
        <p>{{ data.message }}</p>
      </div>
      <div class="confirm-actions">
        <button *ngIf="!data.hideCancel" mat-button (click)="onCancel()">
          {{ data.cancelButton || 'Cancel' }}
        </button>
        <button 
          mat-raised-button 
          [color]="data.danger ? 'warn' : 'primary'"
          (click)="onConfirm()">
          {{ data.confirmButton || 'Confirm' }}
        </button>
      </div>
    </div>
  `,
  styles: [`
    .confirm-dialog {
      padding: 16px;
      width: 100%;
      box-sizing: border-box;
    }
    
    .confirm-header {
      display: flex;
      align-items: center;
      margin-bottom: 16px;
      
      mat-icon {
        margin-right: 8px;
        color: #1976d2;
        
        .warning & {
          color: #f57c00;
        }
        
        .danger & {
          color: #d32f2f;
        }
      }
      
      h2 {
        margin: 0;
        font-size: 20px;
        font-weight: 500;
      }
    }
    
    .confirm-content {
      margin-bottom: 24px;
      
      p {
        margin: 0;
        color: #444;
        line-height: 1.5;
      }
    }
    
    .confirm-actions {
      display: flex;
      justify-content: flex-end;
      gap: 8px;
    }
  `]
})
export class SimpleConfirmDialogComponent {
  constructor(
    public dialogRef: MatDialogRef<SimpleConfirmDialogComponent>,
    @Inject(MAT_DIALOG_DATA) public data: SimpleConfirmDialogData
  ) {}

  onCancel(): void {
    this.dialogRef.close(false);
  }

  onConfirm(): void {
    this.dialogRef.close(true);
  }
  
  // No additional helper methods needed as we're using properties directly in the template
}
