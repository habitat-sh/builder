import { Component, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { MatDialogModule, MatDialogRef, MAT_DIALOG_DATA } from '@angular/material/dialog';
import { MatButtonModule } from '@angular/material/button';
import { ButtonComponent } from '../button/button.component';

export interface DialogData {
  title: string;
  message: string;
  confirmText?: string;
  cancelText?: string;
  type?: 'info' | 'warning' | 'error' | 'confirm';
  hideCancel?: boolean;
}

@Component({
  selector: 'app-dialog',
  standalone: true,
  imports: [CommonModule, MatDialogModule, MatButtonModule, ButtonComponent],
  template: `
    <div class="dialog-container" [ngClass]="data.type">
      <h2 mat-dialog-title>{{ data.title }}</h2>
      
      <mat-dialog-content>
        <div [innerHTML]="data.message"></div>
        <ng-content select="[dialog-content]"></ng-content>
      </mat-dialog-content>
      
      <mat-dialog-actions align="end">
        <ng-content select="[dialog-actions]"></ng-content>
        
        <button 
          *ngIf="!data.hideCancel"
          mat-button 
          (click)="onCancel()">
          {{ data.cancelText || 'Cancel' }}
        </button>
        
        <app-button 
          [color]="getConfirmButtonColor()"
          (buttonClick)="onConfirm()">
          {{ data.confirmText || 'OK' }}
        </app-button>
      </mat-dialog-actions>
    </div>
  `,
  styles: [`
    .dialog-container {
      min-width: 320px;
      max-width: 600px;
    }
    
    mat-dialog-content {
      margin: 16px 0;
    }
    
    h2 {
      margin: 0;
      font-size: 20px;
    }
    
    .warning h2 {
      color: #f57c00;
    }
    
    .error h2 {
      color: #d32f2f;
    }
  `]
})
export class DialogComponent {
  dialogRef = inject(MatDialogRef<DialogComponent>);
  data: DialogData = inject(MAT_DIALOG_DATA);
  
  onConfirm(): void {
    this.dialogRef.close(true);
  }
  
  onCancel(): void {
    this.dialogRef.close(false);
  }
  
  getConfirmButtonColor(): 'primary' | 'accent' | 'warn' | '' {
    switch (this.data.type) {
      case 'warning':
        return 'accent';
      case 'error':
        return 'warn';
      default:
        return 'primary';
    }
  }
}
