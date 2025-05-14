import { Component, OnInit, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterLink } from '@angular/router';
import { MatCardModule } from '@angular/material/card';
import { MatButtonModule } from '@angular/material/button';
import { Title } from '@angular/platform-browser';
import { HeaderService } from '../../../core/services/header.service';
import { HeaderTitleDirective } from '../../../core/layout/shared';

@Component({
  selector: 'app-origins-list',
  standalone: true,
  imports: [CommonModule, RouterLink, MatCardModule, MatButtonModule, HeaderTitleDirective],
  template: `
    <!-- Header Title Template -->
    <ng-template habHeaderTitle>
      <h1>My Origins</h1>
    </ng-template>
    
    <div class="page-container">
      <div class="page-content">
        <mat-card>
          <mat-card-content>
            <p>Origin list will be displayed here.</p>
            <button mat-raised-button color="primary" routerLink="/origins/demo">
              View Demo Origin
            </button>
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
  `]
})
export class OriginsListComponent implements OnInit {
  private title = inject(Title);
  private headerService = inject(HeaderService);

  ngOnInit(): void {
    this.title.setTitle('My Origins | Habitat Builder');
  }
}
