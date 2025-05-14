import { Component, OnInit, inject, signal, effect } from '@angular/core';
import { CommonModule } from '@angular/common';
import { Router, RouterLink } from '@angular/router';
import { MatCardModule } from '@angular/material/card';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';
import { MatTableModule } from '@angular/material/table';
import { MatProgressSpinnerModule } from '@angular/material/progress-spinner';
import { MatDividerModule } from '@angular/material/divider';
import { MatDialogModule, MatDialog } from '@angular/material/dialog';
import { Title } from '@angular/platform-browser';

import { HeaderService } from '../../../core/services/header.service';
import { HeaderTitleDirective } from '../../../core/layout/shared';
import { OriginService } from '../services/origin.service';
import { ConfigService } from '../../../core/services/config.service';
import { DialogService } from '../../../core/services/dialog.service';

import { Origin, OriginInvitation } from '../models/origin.model';

@Component({
  selector: 'app-origins-list',
  standalone: true,
  imports: [
    CommonModule, 
    RouterLink, 
    MatCardModule, 
    MatButtonModule,
    MatIconModule,
    MatTableModule,
    MatProgressSpinnerModule,
    MatDividerModule,
    MatDialogModule,
    HeaderTitleDirective
  ],
  template: `
    <!-- Header Title Template -->
    <ng-template habHeaderTitle>
      <h1>My Origins</h1>
    </ng-template>
    
    <div class="page-container">
      <div class="page-content">
        <!-- Loading Indicator -->
        <div *ngIf="originService.loading()" class="loading-spinner">
          <mat-spinner diameter="40"></mat-spinner>
        </div>
        
        <!-- Error Message -->
        <mat-card *ngIf="originService.error()" class="error-card">
          <mat-card-content>
            <p class="error">{{ originService.error() }}</p>
          </mat-card-content>
        </mat-card>
        
        <!-- Create Origin Section -->
        <mat-card *ngIf="!originService.loading() && !isSaas()">
          <mat-card-content>
            <button mat-raised-button color="primary" routerLink="/origins/create">
              Create Origin
            </button>
          </mat-card-content>
        </mat-card>
        
        <!-- SaaS Notice -->
        <mat-card *ngIf="isSaas()" class="saas-notice">
          <mat-card-content>
            <p class="saas-message">
              <strong>Important Notice:</strong> We would like to inform you that we have disabled the creation 
              of origins in our hosted Chef Habitat Builder (bldr.habitat.sh). However, you can still continue 
              to install an on-prem or self-hosted habitat builder by following these 
              <a href="https://www.chef.io/blog/chef-habitat-product-announcement-builder-on-prem-enhancements-that-extend-support-to-airgap-environments-and-simplify-set-up" target="_blank">instructions</a>. 
              Please <a href="https://www.chef.io/contact-us" target="_blank">contact us</a> if you would like to know more.
            </p>
          </mat-card-content>
        </mat-card>
        
        <!-- No Origins Message -->
        <mat-card *ngIf="!originService.loading() && originService.allOriginItems().length === 0 && !originService.error()">
          <mat-card-content>
            <p>
              <strong>You are not currently an owner or member of any origins.</strong>
            </p>
          </mat-card-content>
        </mat-card>
        
        <!-- Origins List -->
        <mat-card *ngIf="!originService.loading() && originService.allOriginItems().length > 0">
          <mat-card-content>
            <table mat-table [dataSource]="originService.allOriginItems()" class="origins-table">
              <!-- Name Column -->
              <ng-container matColumnDef="name">
                <th mat-header-cell *matHeaderCellDef>Origin Name</th>
                <td mat-cell *matCellDef="let item">{{ originService.getName(item) }}</td>
              </ng-container>
              
              <!-- Package Count Column -->
              <ng-container matColumnDef="packageCount">
                <th mat-header-cell *matHeaderCellDef>Packages</th>
                <td mat-cell *matCellDef="let item">{{ originService.getPackageCount(item) }}</td>
              </ng-container>
              
              <!-- Visibility Column -->
              <ng-container matColumnDef="visibility">
                <th mat-header-cell *matHeaderCellDef>Default Visibility</th>
                <td mat-cell *matCellDef="let item">
                  <mat-icon [title]="originService.getVisibilityLabel(item)">
                    {{ originService.getVisibilityIcon(item) }}
                  </mat-icon>
                </td>
              </ng-container>
              
              <!-- Actions Column -->
              <ng-container matColumnDef="actions">
                <th mat-header-cell *matHeaderCellDef></th>
                <td mat-cell *matCellDef="let item">
                  <div *ngIf="originService.isInvitation(item)" class="invitation-actions">
                    <button mat-button color="primary" (click)="acceptInvitation(item, $event)">
                      <mat-icon>check</mat-icon> Accept
                    </button>
                    <button mat-button color="warn" (click)="ignoreInvitation(item, $event)">
                      <mat-icon>close</mat-icon> Ignore
                    </button>
                  </div>
                  <mat-icon *ngIf="!originService.isInvitation(item)">chevron_right</mat-icon>
                </td>
              </ng-container>
              
              <tr mat-header-row *matHeaderRowDef="displayedColumns"></tr>
              <tr mat-row *matRowDef="let row; columns: displayedColumns;"
                  [ngClass]="{'invitation-row': originService.isInvitation(row)}"
                  (click)="navigateTo(row)"></tr>
            </table>
          </mat-card-content>
        </mat-card>
      </div>
    </div>
  `,
  styles: [`
    .page-container {
      padding: 16px;
    }
    
    .page-header {
      margin-bottom: 16px;
    }
    
    .page-content mat-card {
      margin-bottom: 16px;
    }
    
    .loading-spinner {
      display: flex;
      justify-content: center;
      padding: 2rem;
    }
    
    .error-card {
      border-left: 4px solid #f44336;
    }
    
    .error {
      color: #f44336;
    }
    
    .saas-notice {
      border-left: 4px solid #ff9800;
    }
    
    .saas-message {
      color: #d32f2f;
    }
    
    .origins-table {
      width: 100%;
    }
    
    .invitation-row {
      background-color: rgba(33, 150, 243, 0.05);
    }
    
    .invitation-actions {
      display: flex;
      gap: 8px;
    }
    
    .mat-mdc-row {
      cursor: pointer;
    }
    
    .mat-mdc-row:hover {
      background-color: rgba(0, 0, 0, 0.04);
    }
    
    /* Make sure actions don't trigger row click */
    .invitation-actions button {
      z-index: 2;
      position: relative;
    }
  `]
})
export class OriginsListComponent implements OnInit {
  private title = inject(Title);
  private headerService = inject(HeaderService);
  private router = inject(Router);
  private dialog = inject(MatDialog);
  private dialogService = inject(DialogService);
  protected originService = inject(OriginService);
  private configService = inject(ConfigService);
  
  // Table configuration
  protected displayedColumns: string[] = ['name', 'packageCount', 'visibility', 'actions'];
  
  // Config state
  private isSaasFlag = signal<boolean>(false);
  
  constructor() {
    // Check if application is in SaaS mode
    this.configService.getConfig().subscribe(config => {
      this.isSaasFlag.set(!!config['is_saas']);
    });
  }

  ngOnInit(): void {
    this.title.setTitle('My Origins | Habitat Builder');
    
    // Fetch origins and invitations
    this.originService.initialize();
  }
  
  /**
   * Navigate to origin detail page when row is clicked
   */
  navigateTo(item: Origin | OriginInvitation): void {
    if (!this.originService.isInvitation(item)) {
      this.router.navigate(['/origins', this.originService.getName(item)]);
    }
  }
  
  /**
   * Accept an invitation to join an origin
   */
  acceptInvitation(item: any, event: MouseEvent): void {
    event.stopPropagation(); // Prevent row click navigation
    
    if (this.originService.isInvitation(item)) {
      this.originService.acceptInvitation(item.id, item.origin).subscribe();
    }
  }
  
  /**
   * Ignore an invitation to join an origin
   */
  ignoreInvitation(item: any, event: MouseEvent): void {
    event.stopPropagation(); // Prevent row click navigation
    
    if (this.originService.isInvitation(item)) {
      this.dialogService.confirm(
        'Confirm Ignore Invitation',
        `Are you sure you want to ignore this invitation? Doing so will prevent access to this origin and its private packages.`,
        'Ignore Invitation',
        'Cancel'
      ).subscribe(result => {
        if (result) {
          this.originService.ignoreInvitation(item.id, item.origin).subscribe();
        }
      });
    }
  }
  
  /**
   * Check if the application is in SaaS mode
   */
  isSaas(): boolean {
    return this.isSaasFlag();
  }
}
