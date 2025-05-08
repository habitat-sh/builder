import { Component, EventEmitter, Input, Output } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterModule } from '@angular/router';
import { MatIconModule } from '@angular/material/icon';
import { MatMenuModule } from '@angular/material/menu';
import { MatButtonModule } from '@angular/material/button';

@Component({
  selector: 'app-header',
  standalone: true,
  imports: [
    CommonModule,
    RouterModule,
    MatIconModule,
    MatMenuModule,
    MatButtonModule
  ],
  template: `
    <header class="app-header">
      <div class="logo">
        <a routerLink="/">
          <img src="/assets/images/habitat-logo.svg" alt="Habitat Builder">
          <span class="title">Builder</span>
        </a>
      </div>
      
      <div class="header-nav">
        <a routerLink="/explore" class="nav-item" routerLinkActive="active">Explore</a>
        <a routerLink="/docs" class="nav-item" routerLinkActive="active">Docs</a>
        <a routerLink="/community" class="nav-item" routerLinkActive="active">Community</a>
      </div>
      
      <div class="user-menu">
        @if (isSignedIn) {
          <button mat-button [matMenuTriggerFor]="userMenu" class="user-button">
            @if (avatarUrl) {
              <img [src]="avatarUrl" alt="User Avatar" class="avatar">
            } @else {
              <mat-icon>account_circle</mat-icon>
            }
            <span class="username">{{ username }}</span>
            <mat-icon>arrow_drop_down</mat-icon>
          </button>
          
          <mat-menu #userMenu="matMenu">
            <a mat-menu-item routerLink="/profile">
              <mat-icon>person</mat-icon>
              <span>Profile</span>
            </a>
            <a mat-menu-item routerLink="/origins">
              <mat-icon>business</mat-icon>
              <span>My Origins</span>
            </a>
            <button mat-menu-item (click)="signOut.emit()">
              <mat-icon>exit_to_app</mat-icon>
              <span>Sign Out</span>
            </button>
          </mat-menu>
        } @else {
          <a mat-button routerLink="/sign-in" class="sign-in-button">
            <mat-icon>login</mat-icon>
            Sign In
          </a>
        }
      </div>
    </header>
  `,
  styleUrls: ['./header.component.scss']
})
export class HeaderComponent {
  @Input() isSignedIn = false;
  @Input() username = '';
  @Input() avatarUrl = '';
  @Output() signOut = new EventEmitter<void>();
}
