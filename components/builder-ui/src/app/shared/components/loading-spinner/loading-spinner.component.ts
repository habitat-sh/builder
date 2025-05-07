import { Component, Input } from '@angular/core';
import { CommonModule } from '@angular/common';
import { MatProgressSpinnerModule } from '@angular/material/progress-spinner';

@Component({
  selector: 'app-loading-spinner',
  standalone: true,
  imports: [CommonModule, MatProgressSpinnerModule],
  template: `
    <div class="spinner-container" [ngClass]="{ 'overlay': overlay }">
      <mat-spinner 
        [diameter]="diameter" 
        [color]="color"
        [strokeWidth]="strokeWidth">
      </mat-spinner>
      <div *ngIf="message" class="spinner-message">{{ message }}</div>
    </div>
  `,
  styles: [`
    .spinner-container {
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      padding: 1.5rem;
    }
    
    .spinner-message {
      margin-top: 1rem;
      font-size: 1rem;
      color: rgba(0, 0, 0, 0.7);
    }
    
    .overlay {
      position: fixed;
      top: 0;
      left: 0;
      right: 0;
      bottom: 0;
      background-color: rgba(255, 255, 255, 0.8);
      z-index: 1000;
    }
  `]
})
export class LoadingSpinnerComponent {
  @Input() diameter = 50;
  @Input() strokeWidth = 5;
  @Input() color: 'primary' | 'accent' = 'primary';
  @Input() message?: string;
  @Input() overlay = false;
}
