import { Component, Input, Optional, Self, OnInit } from '@angular/core';
import { CommonModule } from '@angular/common';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatInputModule } from '@angular/material/input';
import { ControlValueAccessor, NgControl, ReactiveFormsModule } from '@angular/forms';
import { MatIconModule } from '@angular/material/icon';

@Component({
  selector: 'app-input',
  standalone: true,
  imports: [CommonModule, MatFormFieldModule, MatInputModule, ReactiveFormsModule, MatIconModule],
  template: `
    <mat-form-field appearance="outline" [class]="formFieldClass">
      <mat-label *ngIf="label">{{ label }}</mat-label>
      
      <input 
        *ngIf="!multiline"
        matInput
        [type]="type"
        [placeholder]="placeholder"
        [attr.aria-label]="label || placeholder"
        [value]="value"
        (input)="onInputChange($event)"
        (blur)="onBlur()"
        [disabled]="disabled"
        [required]="required">
      
      <textarea 
        *ngIf="multiline"
        matInput
        [placeholder]="placeholder"
        [attr.aria-label]="label || placeholder"
        [value]="value"
        (input)="onInputChange($event)"
        (blur)="onBlur()"
        [disabled]="disabled"
        [required]="required"
        [rows]="rows">
      </textarea>
      
      <mat-icon matPrefix *ngIf="prefixIcon" class="prefix-icon">{{ prefixIcon }}</mat-icon>
      <mat-icon matSuffix *ngIf="suffixIcon" class="suffix-icon">{{ suffixIcon }}</mat-icon>
      
      <mat-hint *ngIf="hint">{{ hint }}</mat-hint>
      
      <mat-error *ngIf="control && control.invalid && (control.dirty || control.touched)">
        <ng-container *ngIf="control.errors?.['required']">This field is required</ng-container>
        <ng-container *ngIf="control.errors?.['email']">Please enter a valid email</ng-container>
        <ng-container *ngIf="control.errors?.['minlength']">
          Input is too short (minimum length: {{ control.errors?.['minlength']?.requiredLength }})
        </ng-container>
        <ng-container *ngIf="control.errors?.['maxlength']">
          Input is too long (maximum length: {{ control.errors?.['maxlength']?.requiredLength }})
        </ng-container>
        <ng-container *ngIf="control.errors?.['pattern']">
          Please enter a valid format
        </ng-container>
        <ng-container *ngIf="customErrorMessage">{{ customErrorMessage }}</ng-container>
      </mat-error>
    </mat-form-field>
  `,
  styles: [`
    .prefix-icon, .suffix-icon {
      color: rgba(0, 0, 0, 0.54);
    }
    
    mat-form-field {
      width: 100%;
    }
  `]
})
export class InputComponent implements ControlValueAccessor, OnInit {
  @Input() label = '';
  @Input() placeholder = '';
  @Input() type: 'text' | 'password' | 'email' | 'number' | 'tel' | 'url' = 'text';
  @Input() multiline = false;
  @Input() rows = 3;
  @Input() prefixIcon = '';
  @Input() suffixIcon = '';
  @Input() hint = '';
  @Input() customErrorMessage = '';
  @Input() formFieldClass = '';
  @Input() required = false;
  
  value = '';
  disabled = false;
  
  constructor(@Optional() @Self() public control: NgControl) {
    if (this.control) {
      this.control.valueAccessor = this;
    }
  }
  
  ngOnInit(): void {
    // Check if there's a custom error message from the parent component
    if (this.control && this.control.errors && this.customErrorMessage) {
      // Use custom error message if provided
    }
  }
  
  // ControlValueAccessor methods
  writeValue(value: any): void {
    this.value = value || '';
  }
  
  registerOnChange(fn: any): void {
    this.onChange = fn;
  }
  
  registerOnTouched(fn: any): void {
    this.onTouched = fn;
  }
  
  setDisabledState(isDisabled: boolean): void {
    this.disabled = isDisabled;
  }
  
  onInputChange(event: Event): void {
    const input = event.target as HTMLInputElement | HTMLTextAreaElement;
    this.value = input.value;
    this.onChange(input.value);
  }
  
  onBlur(): void {
    this.onTouched();
  }
  
  // Placeholder methods for ControlValueAccessor
  private onChange: any = () => {};
  private onTouched: any = () => {};
}
