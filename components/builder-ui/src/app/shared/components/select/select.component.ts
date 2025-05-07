import { Component, Input, Output, EventEmitter, Optional, Self } from '@angular/core';
import { CommonModule } from '@angular/common';
import { MatSelectModule } from '@angular/material/select';
import { MatFormFieldModule } from '@angular/material/form-field';
import { ControlValueAccessor, NgControl, FormsModule } from '@angular/forms';

export interface SelectOption {
  value: any;
  label: string;
  disabled?: boolean;
}

@Component({
  selector: 'app-select',
  standalone: true,
  imports: [CommonModule, MatSelectModule, MatFormFieldModule, FormsModule],
  template: `
    <mat-form-field appearance="outline" [class]="formFieldClass">
      <mat-label *ngIf="label">{{ label }}</mat-label>
      
      <mat-select 
        [value]="value"
        [disabled]="disabled"
        [required]="required"
        [multiple]="multiple"
        [placeholder]="placeholder"
        (selectionChange)="onSelectionChange($event)"
        (blur)="onBlur()">
        
        <mat-option *ngIf="showEmptyOption" [value]="null">{{ emptyOptionLabel }}</mat-option>
        
        <mat-optgroup *ngIf="showGroups" [label]="group.label" [disabled]="group.disabled"
          [attr.aria-label]="group.label"
          *ngFor="let group of groups">
          <mat-option 
            *ngFor="let option of group.options" 
            [value]="option.value"
            [disabled]="option.disabled">
            {{ option.label }}
          </mat-option>
        </mat-optgroup>
        
        <mat-option 
          *ngIf="!showGroups"
          *ngFor="let option of options" 
          [value]="option.value"
          [disabled]="option.disabled">
          {{ option.label }}
        </mat-option>
      </mat-select>
      
      <mat-hint *ngIf="hint">{{ hint }}</mat-hint>
      
      <mat-error *ngIf="control && control.invalid && (control.dirty || control.touched)">
        <ng-container *ngIf="control.errors?.['required']">This field is required</ng-container>
        <ng-container *ngIf="customErrorMessage">{{ customErrorMessage }}</ng-container>
      </mat-error>
    </mat-form-field>
  `,
  styles: [`
    mat-form-field {
      width: 100%;
    }
  `]
})
export class SelectComponent implements ControlValueAccessor {
  @Input() options: SelectOption[] = [];
  @Input() groups: { label: string; disabled?: boolean; options: SelectOption[] }[] = [];
  @Input() showGroups = false;
  @Input() label = '';
  @Input() placeholder = '';
  @Input() hint = '';
  @Input() required = false;
  @Input() disabled = false;
  @Input() multiple = false;
  @Input() showEmptyOption = false;
  @Input() emptyOptionLabel = 'Select an option';
  @Input() customErrorMessage = '';
  @Input() formFieldClass = '';
  
  @Output() selectionChange = new EventEmitter<any>();
  
  value: any = '';
  
  constructor(@Optional() @Self() public control: NgControl) {
    if (this.control) {
      this.control.valueAccessor = this;
    }
  }
  
  // ControlValueAccessor methods
  writeValue(value: any): void {
    this.value = value;
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
  
  onSelectionChange(event: any): void {
    this.value = event.value;
    this.onChange(this.value);
    this.selectionChange.emit(this.value);
  }
  
  onBlur(): void {
    this.onTouched();
  }
  
  // Placeholder methods for ControlValueAccessor
  private onChange: any = () => {};
  private onTouched: any = () => {};
}
