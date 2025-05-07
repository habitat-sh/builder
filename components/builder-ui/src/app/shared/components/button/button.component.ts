import { Component, Input, Output, EventEmitter } from '@angular/core';
import { CommonModule } from '@angular/common';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';

@Component({
  selector: 'app-button',
  standalone: true,
  imports: [CommonModule, MatButtonModule, MatIconModule],
  template: `
    <button 
      mat-button
      [color]="color"
      [disabled]="disabled"
      [type]="type"
      [ngClass]="buttonClass"
      (click)="onClick($event)">
      <mat-icon *ngIf="icon">{{ icon }}</mat-icon>
      <ng-content></ng-content>
    </button>
  `,
  styles: [`
    button {
      margin: 0.25rem;
    }
  `]
})
export class ButtonComponent {
  @Input() color: 'primary' | 'accent' | 'warn' | '' = '';
  @Input() disabled = false;
  @Input() type: 'button' | 'submit' | 'reset' = 'button';
  @Input() icon?: string;
  @Input() buttonClass: string = '';
  
  @Output() buttonClick = new EventEmitter<MouseEvent>();
  
  onClick(event: MouseEvent): void {
    this.buttonClick.emit(event);
  }
}
