import { Component } from '@angular/core';
import { RouterOutlet } from '@angular/router';
import { MatSidenavModule } from '@angular/material/sidenav';
import { MatToolbarModule } from '@angular/material/toolbar';
import { MatIconModule } from '@angular/material/icon';
import { MatButtonModule } from '@angular/material/button';
import { MatListModule } from '@angular/material/list';

@Component({
  selector: 'app-layout',
  standalone: true,
  imports: [
    RouterOutlet, 
    MatSidenavModule,
    MatToolbarModule,
    MatIconModule,
    MatButtonModule,
    MatListModule
  ],
  template: `
    <div class="app-container">
      <mat-toolbar color="primary" class="toolbar">
        <button mat-icon-button (click)="sidenavOpened = !sidenavOpened">
          <mat-icon>menu</mat-icon>
        </button>
        <span class="app-title">Habitat Builder</span>
        <span class="spacer"></span>
        <button mat-icon-button aria-label="User menu">
          <mat-icon>account_circle</mat-icon>
        </button>
      </mat-toolbar>

      <mat-sidenav-container class="sidenav-container">
        <mat-sidenav [opened]="sidenavOpened" [mode]="'side'" class="sidenav">
          <mat-nav-list>
            <a mat-list-item routerLink="/">
              <mat-icon>dashboard</mat-icon>
              <span>Dashboard</span>
            </a>
            <a mat-list-item routerLink="/pkgs">
              <mat-icon>inventory_2</mat-icon>
              <span>Packages</span>
            </a>
            <a mat-list-item routerLink="/origins">
              <mat-icon>business</mat-icon>
              <span>Origins</span>
            </a>
            <a mat-list-item routerLink="/builds">
              <mat-icon>build</mat-icon>
              <span>Builds</span>
            </a>
            <a mat-list-item routerLink="/profile">
              <mat-icon>person</mat-icon>
              <span>Profile</span>
            </a>
          </mat-nav-list>
        </mat-sidenav>
        
        <mat-sidenav-content class="content">
          <router-outlet></router-outlet>
        </mat-sidenav-content>
      </mat-sidenav-container>
    </div>
  `,
  styles: [`
    .app-container {
      display: flex;
      flex-direction: column;
      height: 100vh;
    }
    
    .toolbar {
      position: fixed;
      top: 0;
      left: 0;
      right: 0;
      z-index: 2;
    }
    
    .app-title {
      margin-left: 16px;
    }
    
    .spacer {
      flex: 1 1 auto;
    }
    
    .sidenav-container {
      flex: 1;
      margin-top: 64px;
    }
    
    .sidenav {
      width: 250px;
    }
    
    .content {
      padding: 20px;
    }
    
    mat-nav-list a {
      display: flex;
      align-items: center;
    }
    
    mat-nav-list a mat-icon {
      margin-right: 10px;
    }
  `]
})
export class LayoutComponent {
  sidenavOpened = true;
}
