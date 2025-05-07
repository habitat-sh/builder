import { Component, Input, Output, EventEmitter, HostListener, ElementRef } from '@angular/core';
import { CommonModule } from '@angular/common';
import { MatIconModule } from '@angular/material/icon';
import { ButtonComponent } from '../button/button.component';

export interface FileUploadEvent {
  files: File[];
  valid: boolean;
  invalidReason?: string;
}

@Component({
  selector: 'app-file-upload',
  standalone: true,
  imports: [CommonModule, MatIconModule, ButtonComponent],
  template: `
    <div 
      class="file-upload-container" 
      [class.drag-over]="dragOver"
      [class.error]="fileError"
      [class.disabled]="disabled">
      
      <div class="file-upload-dropzone" (click)="openFileDialog()">
        <input 
          #fileInput
          type="file"
          class="file-input"
          [accept]="accept"
          [multiple]="multiple"
          [disabled]="disabled"
          (change)="onFileSelected($event)">
            
        <div class="dropzone-content">
          <mat-icon class="upload-icon">cloud_upload</mat-icon>
          <div class="upload-text">
            <ng-container *ngIf="!selectedFiles?.length">
              <span class="primary-text">{{ primaryText }}</span>
              <span class="secondary-text">{{ secondaryText }}</span>
            </ng-container>
            <ng-container *ngIf="selectedFiles?.length">
              <span class="primary-text">{{ selectedFiles.length }} {{ selectedFiles.length === 1 ? 'file' : 'files' }} selected</span>
              <span class="secondary-text file-names">
                <span *ngFor="let file of selectedFiles">{{ file.name }}</span>
              </span>
            </ng-container>
          </div>
          
          <app-button
            [color]="'primary'"
            [disabled]="disabled">
            {{ buttonText }}
          </app-button>
        </div>
      </div>
      
      <div *ngIf="fileError" class="error-message">
        {{ fileError }}
      </div>
    </div>
  `,
  styles: [`
    .file-upload-container {
      width: 100%;
      border: 2px dashed #ccc;
      border-radius: 4px;
      background-color: #f9f9f9;
      transition: all 0.2s ease;
      margin-bottom: 16px;
    }
    
    .file-upload-dropzone {
      position: relative;
      cursor: pointer;
      padding: 24px;
    }
    
    .file-input {
      position: absolute;
      width: 0;
      height: 0;
      opacity: 0;
      overflow: hidden;
    }
    
    .dropzone-content {
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      text-align: center;
    }
    
    .upload-icon {
      font-size: 48px;
      height: 48px;
      width: 48px;
      color: #757575;
      margin-bottom: 16px;
    }
    
    .upload-text {
      margin-bottom: 16px;
      display: flex;
      flex-direction: column;
    }
    
    .primary-text {
      font-size: 16px;
      font-weight: 500;
      margin-bottom: 8px;
    }
    
    .secondary-text {
      font-size: 14px;
      color: rgba(0, 0, 0, 0.6);
    }
    
    .file-names {
      display: flex;
      flex-direction: column;
      gap: 4px;
    }
    
    .drag-over {
      border-color: #2196f3;
      background-color: rgba(33, 150, 243, 0.05);
    }
    
    .error {
      border-color: #f44336;
    }
    
    .disabled {
      opacity: 0.6;
      cursor: not-allowed;
    }
    
    .error-message {
      color: #f44336;
      font-size: 12px;
      padding: 8px 16px;
      background-color: rgba(244, 67, 54, 0.1);
      border-top: 1px solid rgba(244, 67, 54, 0.2);
    }
  `]
})
export class FileUploadComponent {
  @Input() multiple = false;
  @Input() accept = '';
  @Input() maxFileSize = 0; // in bytes, 0 means unlimited
  @Input() disabled = false;
  @Input() primaryText = 'Drag and drop files here';
  @Input() secondaryText = 'or click to browse';
  @Input() buttonText = 'Browse Files';
  
  @Output() filesSelected = new EventEmitter<FileUploadEvent>();
  
  dragOver = false;
  fileError = '';
  selectedFiles: File[] = [];
  
  constructor(private elementRef: ElementRef) { }
  
  @HostListener('dragover', ['$event'])
  onDragOver(event: DragEvent): void {
    if (this.disabled) {
      return;
    }
    
    event.preventDefault();
    event.stopPropagation();
    this.dragOver = true;
  }
  
  @HostListener('dragleave', ['$event'])
  onDragLeave(event: DragEvent): void {
    event.preventDefault();
    event.stopPropagation();
    this.dragOver = false;
  }
  
  @HostListener('drop', ['$event'])
  onDrop(event: DragEvent): void {
    if (this.disabled) {
      return;
    }
    
    event.preventDefault();
    event.stopPropagation();
    this.dragOver = false;
    
    const files = event.dataTransfer?.files;
    if (files && files.length > 0) {
      this.handleFiles(Array.from(files));
    }
  }
  
  openFileDialog(): void {
    if (this.disabled) {
      return;
    }
    
    const fileInput = this.elementRef.nativeElement.querySelector('input[type="file"]');
    if (fileInput) {
      fileInput.click();
    }
  }
  
  onFileSelected(event: Event): void {
    const input = event.target as HTMLInputElement;
    if (input.files && input.files.length > 0) {
      this.handleFiles(Array.from(input.files));
    }
  }
  
  private handleFiles(files: File[]): void {
    this.fileError = '';
    
    // Check file count for multiple input
    if (!this.multiple && files.length > 1) {
      this.fileError = 'Only one file can be selected';
      this.emitFileEvent([], false, this.fileError);
      return;
    }
    
    // Check file extensions if accept is set
    if (this.accept) {
      const acceptedTypes = this.accept.split(',').map(type => type.trim().toLowerCase());
      const invalidFiles = files.filter(file => {
        const extension = '.' + file.name.split('.').pop()?.toLowerCase();
        const mimeType = file.type.toLowerCase();
        
        return !acceptedTypes.some(type => {
          if (type.startsWith('.')) {
            // Extension check
            return extension === type;
          } else if (type.endsWith('/*')) {
            // MIME type group check (e.g., image/*)
            const group = type.split('/')[0];
            return mimeType.startsWith(group + '/');
          } else {
            // Exact MIME type check
            return mimeType === type;
          }
        });
      });
      
      if (invalidFiles.length > 0) {
        this.fileError = `Invalid file type${invalidFiles.length > 1 ? 's' : ''}: ${invalidFiles.map(f => f.name).join(', ')}`;
        this.emitFileEvent([], false, this.fileError);
        return;
      }
    }
    
    // Check file size if maxFileSize is set
    if (this.maxFileSize > 0) {
      const oversizedFiles = files.filter(file => file.size > this.maxFileSize);
      
      if (oversizedFiles.length > 0) {
        const maxSizeMB = this.maxFileSize / (1024 * 1024);
        this.fileError = `File${oversizedFiles.length > 1 ? 's' : ''} exceed${oversizedFiles.length === 1 ? 's' : ''} the maximum size of ${maxSizeMB.toFixed(2)} MB`;
        this.emitFileEvent([], false, this.fileError);
        return;
      }
    }
    
    // All validations passed
    this.selectedFiles = files;
    this.emitFileEvent(files, true);
  }
  
  private emitFileEvent(files: File[], valid: boolean, invalidReason?: string): void {
    this.filesSelected.emit({
      files,
      valid,
      invalidReason
    });
  }
}
