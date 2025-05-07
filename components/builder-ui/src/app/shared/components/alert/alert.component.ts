import { Component, Input } from '@angular/core';
import { CommonModule } from '@angular/common';
import { MatIconModule } from '@angular/material/icon';

@Component({
  selector: 'app-alert',
  standalone: true,
  imports: [CommonModule, MatIconModule],
  template: `
    <div 
      class="alert" 
      [ngClass]="{
        'alert-info': type === 'info',
        'alert-success': type === 'success',
        'alert-warning': type === 'warning',
        'alert-error': type === 'error'
      }"
      *ngIf="isVisible">
      
      <mat-icon class="alert-icon">{{ getIconForType() }}</mat-icon>
      
      <div class="alert-content">
        <div class="alert-title" *ngIf="title">{{ title }}</div>
        <div class="alert-message">
          <ng-content></ng-content>
        </div>
      </div>
      
      <button class="close-button" *ngIf="dismissible" (click)="close()">
        <mat-icon>close</mat-icon>
      </button>
    </div>
  `,
  styles: [`
    .alert {
      display: flex;
      align-items: flex-start;
      padding: 1rem;
      margin-bottom: 1rem;
      border-radius: 4px;
      box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
    }
    
    .alert-info {
      background-color: #e3f2fd;
      border-left: 4px solid #2196f3;
    }
    
    .alert-success {
      background-color: #e8f5e9;
      border-left: 4px solid #4caf50;
    }
    
    .alert-warning {
      background-color: #fff8e1;
      border-left: 4px solid #ff9800;
    }
    
    .alert-error {
      background-color: #ffebee;
      border-left: 4px solid #f44336;
    }
    
    .alert-icon {
      margin-right: 1rem;
      font-size: 24px;
      width: 24px;
      height: 24px;
    }
    
    .alert-info .alert-icon {
      color: #2196f3;
    }
    
    .alert-success .alert-icon {
      color: #4caf50;
    }
    
    .alert-warning .alert-icon {
      color: #ff9800;
    }
    
    .alert-error .alert-icon {
      color: #f44336;
    }
    
    .alert-content {
      flex: 1;
    }
    
    .alert-title {
      font-weight: bold;
      margin-bottom: 0.25rem;
    }
    
    .close-button {
      background: none;
      border: none;
      cursor: pointer;
      opacity: 0.7;
      transition: opacity 0.2s;
      padding: 0;
    }
    
    .close-button:hover {
      opacity: 1;
    }
  `]
})
export class AlertComponent {
  @Input() type: 'info' | 'success' | 'warning' | 'error' = 'info';
  @Input() title?: string;
  @Input() dismissible = false;
  
  isVisible = true;
  
  close(): void {
    this.isVisible = false;
  }
  
  getIconForType(): string {
    switch(this.type) {
      case 'info':
        return 'info';
      case 'success':
        return 'check_circle';
      case 'warning':
        return 'warning';
      case 'error':
        return 'error';
      default:
        return 'info';
    }
  }
}
