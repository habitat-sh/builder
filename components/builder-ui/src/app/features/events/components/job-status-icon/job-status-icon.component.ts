import { Component, Input } from '@angular/core';
import { CommonModule } from '@angular/common';
import { MatIconModule } from '@angular/material/icon';
import { MatTooltipModule } from '@angular/material/tooltip';
import { iconForJobState, labelForJobState } from '../../utils/event-util';

@Component({
  selector: 'app-job-status-icon',
  standalone: true,
  imports: [
    CommonModule,
    MatIconModule,
    MatTooltipModule
  ],
  template: `
    <mat-icon 
      [ngClass]="classes" 
      [matTooltip]="label" 
      aria-hidden="false" 
      [attr.aria-label]="label">
      {{ symbol }}
    </mat-icon>
  `,
  styles: [`
    mat-icon {
      font-size: 18px;
      height: 18px;
      width: 18px;
      vertical-align: middle;
      
      // Status-based colors
      &.canceled, &.cancelpending, &.cancelprocessing, &.cancelcomplete, &.skipped {
        color: #999; // Medium gray for skipped/canceled
      }
      
      &.complete, &.success, &.promote {
        color: #87B09A; // Success green
      }
      
      &.dispatching, &.dispatched, &.inprogress, &.processing, &.queued, &.notstarted, &.pending {
        color: #4296b4; // Info blue
      }
      
      &.failed, &.failure, &.rejected, &.demote {
        color: #EB6852; // Error red
      }
      
      &.animate {
        animation: spin 2s infinite linear;
      }
    }
    
    @keyframes spin {
      from { transform: rotate(0deg); }
      to { transform: rotate(360deg); }
    }
  `]
})
export class JobStatusIconComponent {
  @Input() job: any;
  @Input() status = '';
  @Input() animate = false;

  get _status(): string {
    return this.status || (this.job && this.job.state ? this.job.state : '');
  }

  get classes(): string[] {
    const c = [this._status.toLowerCase()];

    if (this.animate && ['dispatching', 'processing', 'inprogress'].includes(this._status.toLowerCase())) {
      c.push('animate');
    }

    return c;
  }

  get symbol(): string {
    if (this._status) {
      return iconForJobState(this._status);
    }
    return 'help';
  }

  get label(): string {
    if (this._status) {
      return labelForJobState(this._status);
    }
    return 'Unknown status';
  }
}
