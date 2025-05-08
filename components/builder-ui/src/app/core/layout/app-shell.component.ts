import { Component, OnInit, inject, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterModule } from '@angular/router';

// Import header, sidebar and footer components with proper relative paths
import { HeaderComponent } from './header/header.component';
import { SidebarComponent } from './sidebar/sidebar.component';
import { FooterComponent } from './footer/footer.component'; 
import { AuthService } from '../services/auth.service';

@Component({
  selector: 'app-shell',
  standalone: true,
  imports: [
    CommonModule,
    RouterModule,
    HeaderComponent,
    SidebarComponent,
    FooterComponent
  ],
  template: `
    <div class="app-shell">
      <div class="wrapper">
        <nav class="menu" [class.open]="menuOpen()">
          <app-sidebar 
            [isSignedIn]="isSignedIn()" 
            (closeMobileSidebar)="toggleMenu(false)">
          </app-sidebar>
        </nav>
        <main>
          <div class="menu-toggle" (click)="toggleMenu()">
            <span class="sr-only">Toggle menu</span>
          </div>
          <app-header 
            [isSignedIn]="isSignedIn()" 
            [username]="username()"
            [avatarUrl]="avatarUrl()"
          (signOut)="handleSignOut()">
        </app-header>
        <div class="content-container">
          <router-outlet></router-outlet>
        </div>
        <app-footer></app-footer>
      </main>
    </div>
  </div>
  `,
  styleUrls: ['./app-shell.component.scss']
})
export class AppShellComponent implements OnInit {
  private authService = inject(AuthService);
  
  menuOpen = signal<boolean>(false);
  isSignedIn = signal<boolean>(false);
  username = signal<string>('');
  avatarUrl = signal<string>('');
  
  ngOnInit() {
    // Set initial state based on authService
    this.isSignedIn.set(this.authService.isAuthenticated());
    
    if (this.authService.isAuthenticated()) {
      const user = this.authService.currentUser();
      if (user) {
        this.username.set(user.name);
        this.avatarUrl.set(user.avatar || '');
      }
    }
    
    // Subscribe to auth state changes using the legacy observable
    this.authService.authStatus$.subscribe(isAuth => {
      this.isSignedIn.set(isAuth);
      
      if (isAuth) {
        const user = this.authService.currentUser();
        if (user) {
          this.username.set(user.name);
          this.avatarUrl.set(user.avatar || '');
        }
      } else {
        this.username.set('');
        this.avatarUrl.set('');
      }
    });
  }
  
  toggleMenu(forcedState?: boolean) {
    const newState = forcedState !== undefined ? forcedState : !this.menuOpen();
    this.menuOpen.set(newState);
  }
  
  handleSignOut() {
    // Use the AuthService logout method
    this.authService.logout();
    // Ensure our local state is updated
    this.isSignedIn.set(false);
    this.username.set('');
    this.avatarUrl.set('');
  }
}
