import { Component, Input } from '@angular/core';
import { CommonModule } from '@angular/common';
import { MatCardModule } from '@angular/material/card';

@Component({
  selector: 'app-card',
  standalone: true,
  imports: [CommonModule, MatCardModule],
  template: `
    <mat-card [ngClass]="cardClass">
      <mat-card-header *ngIf="title || subtitle">
        <mat-card-title *ngIf="title">{{ title }}</mat-card-title>
        <mat-card-subtitle *ngIf="subtitle">{{ subtitle }}</mat-card-subtitle>
      </mat-card-header>
      
      <mat-card-content>
        <ng-content select="[card-content]"></ng-content>
      </mat-card-content>
      
      <mat-card-actions *ngIf="hasActions">
        <ng-content select="[card-actions]"></ng-content>
      </mat-card-actions>
      
      <mat-card-footer *ngIf="hasFooter">
        <ng-content select="[card-footer]"></ng-content>
      </mat-card-footer>
    </mat-card>
  `,
  styles: [`
    mat-card {
      margin-bottom: 1rem;
    }
  `]
})
export class CardComponent {
  @Input() title?: string;
  @Input() subtitle?: string;
  @Input() cardClass = '';
  @Input() hasActions = false;
  @Input() hasFooter = false;
}
