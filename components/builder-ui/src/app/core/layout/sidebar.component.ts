import { Component, EventEmitter, Input, Output } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterModule } from '@angular/router';
import { MatIconModule } from '@angular/material/icon';
import { MatDividerModule } from '@angular/material/divider';

@Component({
  selector: 'app-sidebar',
  standalone: true,
  imports: [
    CommonModule,
    RouterModule,
    MatIconModule,
    MatDividerModule
  ],
  template: `
    <div class="sidebar">
      <div class="sidebar-header">
        <h1>Habitat</h1>
        @if (isMobileView) {
          <button class="close-button" (click)="closeMobileSidebar.emit()">
            <mat-icon>close</mat-icon>
          </button>
        }
      </div>
      
      <nav class="sidebar-nav">
        <a routerLink="/dashboard" routerLinkActive="active" class="nav-item">
          <mat-icon>dashboard</mat-icon>
          <span>Dashboard</span>
        </a>
        
        <a routerLink="/pkgs" routerLinkActive="active" class="nav-item">
          <mat-icon>inventory_2</mat-icon>
          <span>Packages</span>
        </a>
        
        @if (isSignedIn) {
          <a routerLink="/origins" routerLinkActive="active" class="nav-item">
            <mat-icon>business</mat-icon>
            <span>Origins</span>
          </a>
          
          <a routerLink="/builds" routerLinkActive="active" class="nav-item">
            <mat-icon>build</mat-icon>
            <span>Builds</span>
          </a>
        }
        
        <mat-divider></mat-divider>
        
        <a href="https://www.habitat.sh/docs" target="_blank" class="nav-item">
          <mat-icon>library_books</mat-icon>
          <span>Documentation</span>
        </a>
        
        <a href="https://www.habitat.sh/tutorials" target="_blank" class="nav-item">
          <mat-icon>school</mat-icon>
          <span>Tutorials</span>
        </a>
        
        <a href="https://www.habitat.sh/community" target="_blank" class="nav-item">
          <mat-icon>people</mat-icon>
          <span>Community</span>
        </a>
      </nav>
    </div>
  `,
  styleUrls: ['./sidebar.component.scss']
})
export class SidebarComponent {
  @Input() isSignedIn = false;
  @Output() closeMobileSidebar = new EventEmitter<void>();
  
  get isMobileView(): boolean {
    return window.innerWidth < 768;
  }
}
