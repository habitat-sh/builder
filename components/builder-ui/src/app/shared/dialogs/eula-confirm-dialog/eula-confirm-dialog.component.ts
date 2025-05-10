import { Component, Inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { MatDialogRef, MAT_DIALOG_DATA, MatDialogModule } from '@angular/material/dialog';
import { MatButtonModule } from '@angular/material/button';
import { MatCheckboxModule } from '@angular/material/checkbox';
import { MatIconModule } from '@angular/material/icon';

export interface DialogData {
  heading: string;
  action: string;
  signupUrl: string;
}

@Component({
  selector: 'app-eula-confirm-dialog',
  standalone: true,
  imports: [CommonModule, FormsModule, MatDialogModule, MatButtonModule, MatCheckboxModule, MatIconModule],
  template: `
    <div class="dialog">
      <section class="eula-heading">
        <h1>{{ heading }}</h1>
        <a class="close-button" (click)="cancel()">
          <mat-icon>close</mat-icon>
        </a>
      </section>
      <section class="body">
        <span class="inner-body">
          <div class="checkbox">
            <mat-checkbox (change)="checkbox()" [(ngModel)]="checked"></mat-checkbox>
          </div>
          <div class="message">
            <p>
              I acknowledge and agree that use of Progress Chef Habitat Builder is governed by and subject to the terms and conditions of the End User License Agreement for 
              Progress Chef located at <a href="https://www.chef.io/end-user-license-agreement" target="_blank">Progress Chef EULA</a>
            </p>
          </div>
        </span>
      </section>
      <section class="controls">
        <button mat-raised-button color="primary" class="button continue-button" (click)="ok()" [disabled]="isButtonDisabled">
          {{ action }}
        </button>
      </section>
    </div>
  `,
  styles: [`
    .dialog {
      padding: 0 24px 24px 24px;
      min-width: 320px;
    }
    
    .eula-heading {
      margin: -24px -24px 0px -24px;
      border-bottom: #e7ebed 1px solid;
      padding-left: 24px; 
      display: flex; 
      justify-content: space-between; 
      align-items: center;
    }

    .close-button {
      margin-right: 24px;
      cursor: pointer;
    }

    .continue-button {
      width: 100%; 
      height: 3rem;
    }

    .message {
      margin-left: 24px;
      margin-top: 3px;
    }

    .checkbox {
      padding-top: 5px;
    }

    .inner-body {
      display: flex;
      margin-top: 20px;
      margin-bottom: 20px;
    }
    
    a {
      color: #007fab;
      text-decoration: none;
    }
    
    a:hover {
      text-decoration: underline;
    }
    
    .controls {
      margin-top: 10px;
    }
  `]
})
export class EulaConfirmDialogComponent {
  isButtonDisabled: boolean = true;
  checked: boolean = false;

  constructor(
    public dialogRef: MatDialogRef<EulaConfirmDialogComponent>,
    @Inject(MAT_DIALOG_DATA) public data: DialogData
  ) {}

  get heading() {
    return this.data.heading || 'Confirm';
  }

  get action() {
    return this.data.action || 'Continue';
  }

  get signupUrl() {
    return this.data.signupUrl;
  }

  ok() {
    this.dialogRef.close(true);
  }

  checkbox() {
    this.isButtonDisabled = !this.checked;
  }

  cancel() {
    this.dialogRef.close(false);
  }
}
