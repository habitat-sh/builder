import { Component, OnInit } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterLink } from '@angular/router';
import { MatCardModule } from '@angular/material/card';
import { MatButtonModule } from '@angular/material/button';
import { Title } from '@angular/platform-browser';

@Component({
  selector: 'app-origins-list',
  standalone: true,
  imports: [CommonModule, RouterLink, MatCardModule, MatButtonModule],
  template: `
    <div class="page-container">
      <div class="page-header">
        <h1>Origins</h1>
      </div>
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
  
  constructor(private title: Title) { }

  ngOnInit(): void {
    this.title.setTitle('Origins | Habitat Builder');
  }
}
