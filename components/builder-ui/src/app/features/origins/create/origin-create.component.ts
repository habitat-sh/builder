import { Component, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormBuilder, FormGroup, ReactiveFormsModule, Validators } from '@angular/forms';
import { MatCardModule } from '@angular/material/card';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatInputModule } from '@angular/material/input';
import { MatButtonModule } from '@angular/material/button';
import { MatSelectModule } from '@angular/material/select';
import { MatRadioModule } from '@angular/material/radio';
import { Router } from '@angular/router';

import { OriginService } from '../services/origin.service';
import { HeaderTitleDirective } from '../../../core/layout/shared';
import { Title } from '@angular/platform-browser';

@Component({
  selector: 'app-origin-create',
  standalone: true,
  imports: [
    CommonModule,
    ReactiveFormsModule,
    MatCardModule,
    MatFormFieldModule,
    MatInputModule,
    MatButtonModule,
    MatSelectModule,
    MatRadioModule,
    HeaderTitleDirective
  ],
  template: `
    <ng-template habHeaderTitle>
      <h1>Create Origin</h1>
    </ng-template>

    <div class="page-container">
      <div class="page-content">
        <mat-card>
          <mat-card-content>
            <form [formGroup]="originForm" (ngSubmit)="onSubmit()">
              <h2>Origin Details</h2>
              
              <mat-form-field appearance="outline" class="full-width">
                <mat-label>Origin Name</mat-label>
                <input matInput formControlName="name" placeholder="Enter origin name" autocomplete="off">
                <mat-error *ngIf="originForm.get('name')?.hasError('required')">
                  Origin name is required
                </mat-error>
                <mat-error *ngIf="originForm.get('name')?.hasError('pattern')">
                  Origin name must contain only lowercase letters, numbers, and hyphens
                </mat-error>
                <mat-hint>Names can contain only lowercase letters, numbers, and hyphens</mat-hint>
              </mat-form-field>
              
              <div class="visibility-section">
                <h3>Default Package Visibility</h3>
                <mat-radio-group formControlName="default_package_visibility" class="radio-group">
                  <mat-radio-button value="public">Public</mat-radio-button>
                  <mat-radio-button value="private">Private</mat-radio-button>
                </mat-radio-group>
                <p class="hint">
                  <strong>Public:</strong> Packages will be visible and accessible to all users.<br>
                  <strong>Private:</strong> Packages will only be visible to origin members.
                </p>
              </div>
              
              <div class="form-actions">
                <button mat-button type="button" (click)="cancel()">Cancel</button>
                <button mat-raised-button color="primary" type="submit" [disabled]="originForm.invalid || submitting">
                  Create Origin
                </button>
              </div>
            </form>
          </mat-card-content>
        </mat-card>
      </div>
    </div>
  `,
  styles: [`
    .page-container {
      padding: 16px;
    }
    
    .full-width {
      width: 100%;
    }
    
    .visibility-section {
      margin: 24px 0;
    }
    
    .radio-group {
      display: flex;
      flex-direction: column;
      gap: 12px;
      margin-bottom: 12px;
    }
    
    .hint {
      color: rgba(0, 0, 0, 0.6);
      font-size: 14px;
    }
    
    .form-actions {
      display: flex;
      justify-content: flex-end;
      gap: 12px;
      margin-top: 32px;
    }
  `]
})
export class OriginCreateComponent {
  private fb = inject(FormBuilder);
  private originService = inject(OriginService);
  private router = inject(Router);
  private title = inject(Title);
  
  originForm: FormGroup;
  submitting = false;
  
  constructor() {
    this.title.setTitle('Create Origin | Habitat Builder');
    
    this.originForm = this.fb.group({
      name: ['', [
        Validators.required, 
        Validators.pattern(/^[a-z0-9][a-z0-9_-]*$/)
      ]],
      default_package_visibility: ['public', Validators.required]
    });
  }
  
  onSubmit() {
    if (this.originForm.valid) {
      this.submitting = true;
      
      this.originService.createOrigin(this.originForm.value).subscribe({
        next: () => {
          this.submitting = false;
          this.router.navigate(['/origins']);
        },
        error: (err) => {
          this.submitting = false;
          console.error('Failed to create origin:', err);
        }
      });
    }
  }
  
  cancel() {
    this.router.navigate(['/origins']);
  }
}
