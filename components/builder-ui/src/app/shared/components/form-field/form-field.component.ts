import { Component, Input, ContentChild, Optional, Self } from '@angular/core';
import { CommonModule } from '@angular/common';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatInputModule } from '@angular/material/input';
import { NgControl, FormGroupDirective, ReactiveFormsModule } from '@angular/forms';
import { InputComponent } from '../input/input.component';

@Component({
  selector: 'app-form-field',
  standalone: true,
  imports: [CommonModule, MatFormFieldModule, MatInputModule, ReactiveFormsModule],
  template: `
    <div class="form-field-container" [class.has-error]="showError">
      <label *ngIf="label" [for]="controlId" class="form-field-label">
        {{ label }}
        <span *ngIf="required" class="required-indicator">*</span>
      </label>
      
      <div class="form-field-content">
        <ng-content></ng-content>
        
        <div *ngIf="showError && errorMessage" class="error-message">
          {{ errorMessage }}
        </div>
        
        <div *ngIf="hint && !showError" class="hint-text">
          {{ hint }}
        </div>
      </div>
    </div>
  `,
  styles: [`
    .form-field-container {
      margin-bottom: 16px;
    }
    
    .form-field-label {
      display: block;
      margin-bottom: 6px;
      font-weight: 500;
      font-size: 14px;
      color: rgba(0, 0, 0, 0.87);
    }
    
    .required-indicator {
      color: #f44336;
      margin-left: 4px;
    }
    
    .error-message {
      color: #f44336;
      font-size: 12px;
      margin-top: 4px;
    }
    
    .hint-text {
      color: rgba(0, 0, 0, 0.6);
      font-size: 12px;
      margin-top: 4px;
    }
    
    .has-error input, 
    .has-error select, 
    .has-error textarea {
      border-color: #f44336;
    }
  `]
})
export class FormFieldComponent {
  @Input() label = '';
  @Input() hint = '';
  @Input() required = false;
  @Input() controlId = '';
  
  @ContentChild(NgControl) control?: NgControl;
  
  constructor(@Optional() private formGroup: FormGroupDirective) { }
  
  get showError(): boolean {
    if (!this.control) {
      return false;
    }
    
    const control = this.control.control;
    if (!control) {
      return false;
    }
    
    return (control.invalid && (control.touched || control.dirty))
      || (this.formGroup && this.formGroup.submitted && control.invalid);
  }
  
  get errorMessage(): string {
    if (!this.control || !this.control.errors) {
      return '';
    }
    
    const errors = this.control.errors;
    
    if (errors['required']) {
      return 'This field is required';
    }
    
    if (errors['email']) {
      return 'Please enter a valid email address';
    }
    
    if (errors['minlength']) {
      const requiredLength = errors['minlength'].requiredLength;
      return `Must be at least ${requiredLength} characters`;
    }
    
    if (errors['maxlength']) {
      const requiredLength = errors['maxlength'].requiredLength;
      return `Cannot be more than ${requiredLength} characters`;
    }
    
    if (errors['pattern']) {
      return 'Please enter a valid format';
    }
    
    if (errors['min']) {
      return `Value must be at least ${errors['min'].min}`;
    }
    
    if (errors['max']) {
      return `Value cannot be more than ${errors['max'].max}`;
    }
    
    // Return a generic error message or the first error key as fallback
    return errors[Object.keys(errors)[0]] || 'Invalid value';
  }
}
