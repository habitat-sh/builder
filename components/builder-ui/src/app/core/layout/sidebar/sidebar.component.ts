import { Component, Input } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterLink, RouterLinkActive } from '@angular/router';
import { MatListModule } from '@angular/material/list';
import { MatIconModule } from '@angular/material/icon';
import { MatDividerModule } from '@angular/material/divider';
import { MatTooltipModule } from '@angular/material/tooltip';

export interface NavigationItem {
  label: string;
  icon?: string;
  route?: string;
  children?: NavigationItem[];
  expanded?: boolean;
  divider?: boolean;
  permissions?: string[];
}

@Component({
  selector: 'app-sidebar',
  standalone: true,
  imports: [
    CommonModule,
    RouterLink,
    RouterLinkActive,
    MatListModule,
    MatIconModule,
    MatDividerModule,
    MatTooltipModule
  ],
  template: `
    <div class="sidebar-container">
      <!-- App logo -->
      <div class="logo-container" *ngIf="showLogo">
        <img 
          [src]="logoUrl"
          [alt]="appName"
          class="logo">
        <h2 class="app-name" *ngIf="!collapsed">{{ appName }}</h2>
      </div>
      
      <!-- Navigation items -->
      <div class="nav-items">
        <ng-container *ngFor="let item of navigationItems">
          <!-- Divider -->
          <mat-divider *ngIf="item.divider"></mat-divider>
          
          <!-- Regular navigation item with link -->
          <a 
            *ngIf="!item.divider && item.route && !item.children?.length"
            mat-list-item
            [routerLink]="item.route"
            routerLinkActive="active-link"
            class="nav-item"
            [matTooltip]="collapsed ? item.label : ''"
            [matTooltipPosition]="'right'">
            <mat-icon *ngIf="item.icon" matListItemIcon>{{ item.icon }}</mat-icon>
            <span matListItemTitle *ngIf="!collapsed">{{ item.label }}</span>
          </a>
          
          <!-- Group with children -->
          <div *ngIf="!item.divider && item.children?.length" class="nav-group">
            <div 
              class="nav-group-header"
              [class.active]="item.expanded"
              (click)="toggleGroup(item)">
              <mat-icon *ngIf="item.icon">{{ item.icon }}</mat-icon>
              <span *ngIf="!collapsed">{{ item.label }}</span>
              <span class="spacer"></span>
              <mat-icon *ngIf="!collapsed" class="expand-icon">
                {{ item.expanded ? 'expand_less' : 'expand_more' }}
              </mat-icon>
            </div>
            
            <div class="nav-group-items" *ngIf="item.expanded && !collapsed">
              <a 
                *ngFor="let child of item.children"
                mat-list-item
                [routerLink]="child.route"
                routerLinkActive="active-child"
                class="nav-child-item">
                <mat-icon *ngIf="child.icon" matListItemIcon>{{ child.icon }}</mat-icon>
                <span matListItemTitle>{{ child.label }}</span>
              </a>
            </div>
          </div>
        </ng-container>
      </div>
      
      <!-- Footer slot -->
      <div class="sidebar-footer" *ngIf="!collapsed">
        <ng-content select="[sidebar-footer]"></ng-content>
      </div>
    </div>
  `,
  styles: [`
    .sidebar-container {
      display: flex;
      flex-direction: column;
      height: 100%;
      overflow: hidden;
    }
    
    .logo-container {
      display: flex;
      align-items: center;
      padding: 16px;
      border-bottom: 1px solid rgba(0, 0, 0, 0.12);
    }
    
    .logo {
      height: 32px;
      width: auto;
    }
    
    .app-name {
      margin: 0 0 0 8px;
      font-weight: 400;
      font-size: 16px;
    }
    
    .nav-items {
      flex: 1;
      overflow-y: auto;
      padding: 8px 0;
    }
    
    .nav-item {
      height: 48px;
      padding: 0 16px;
      display: flex;
      align-items: center;
      transition: background-color 0.3s;
    }
    
    .active-link {
      background-color: rgba(33, 150, 243, 0.1);
      color: #1976d2;
      border-left: 4px solid #1976d2;
    }
    
    .nav-group {
      margin: 8px 0;
    }
    
    .nav-group-header {
      display: flex;
      align-items: center;
      padding: 0 16px;
      height: 48px;
      cursor: pointer;
      transition: background-color 0.3s;
    }
    
    .nav-group-header:hover {
      background-color: rgba(0, 0, 0, 0.04);
    }
    
    .nav-group-header.active {
      background-color: rgba(0, 0, 0, 0.04);
    }
    
    .nav-group-header mat-icon {
      margin-right: 16px;
    }
    
    .nav-group-items {
      padding-left: 16px;
    }
    
    .nav-child-item {
      height: 40px;
      font-size: 14px;
    }
    
    .active-child {
      background-color: rgba(33, 150, 243, 0.05);
      color: #1976d2;
    }
    
    .expand-icon {
      transition: transform 0.3s;
      font-size: 18px;
      width: 18px;
      height: 18px;
    }
    
    .spacer {
      flex: 1;
    }
    
    .sidebar-footer {
      border-top: 1px solid rgba(0, 0, 0, 0.12);
      padding: 16px;
    }
  `]
})
export class SidebarComponent {
  @Input() appName = 'Habitat Builder';
  @Input() logoUrl = '/assets/images/habitat-logo.svg';
  @Input() collapsed = false;
  @Input() showLogo = true;
  @Input() navigationItems: NavigationItem[] = [];
  
  toggleGroup(item: NavigationItem): void {
    if (!this.collapsed) {
      item.expanded = !item.expanded;
    }
  }
}
