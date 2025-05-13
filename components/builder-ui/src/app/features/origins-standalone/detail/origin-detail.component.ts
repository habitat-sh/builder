import { Component, OnInit, OnDestroy, ViewChild } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ActivatedRoute, RouterLink, RouterModule, RouterOutlet } from '@angular/router';
import { MatCardModule } from '@angular/material/card';
import { MatTabsModule } from '@angular/material/tabs';
import { Title } from '@angular/platform-browser';
import { Subscription } from 'rxjs';

@Component({
  selector: 'app-origin-detail',
  standalone: true,
  imports: [CommonModule, RouterLink, RouterModule, RouterOutlet, MatCardModule, MatTabsModule],
  template: `
    <div class="page-container">
      <div class="page-header">
        <h1>{{ originName }}</h1>
      </div>
      <div class="page-content">
        <mat-card>
          <mat-card-content>
            <nav mat-tab-nav-bar [tabPanel]="tabPanel">
              <a mat-tab-link
                 [routerLink]="['/origins', originName]"
                 [routerLinkActiveOptions]="{exact: true}"
                 routerLinkActive="active-link">Overview</a>
              <a mat-tab-link
                 [routerLink]="['/origins', originName, 'settings']"
                 routerLinkActive="active-link">Settings</a>
              <a mat-tab-link
                 [routerLink]="['/origins', originName, 'members']"
                 routerLinkActive="active-link">Members</a>
              <a mat-tab-link
                 [routerLink]="['/origins', originName, 'keys']"
                 routerLinkActive="active-link">Keys</a>
              <a mat-tab-link
                 [routerLink]="['/origins', originName, 'integrations']"
                 routerLinkActive="active-link">Integrations</a>
            </nav>
            
            <mat-tab-nav-panel #tabPanel>
              <div class="tab-content">
                <router-outlet></router-outlet>
              </div>
            </mat-tab-nav-panel>
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
    
    .tab-content {
      padding: 16px 8px;
    }
    
    .active-link {
      font-weight: bold;
    }
  `]
})
export class OriginDetailComponent implements OnInit, OnDestroy {
  originName: string = '';
  private subscription: Subscription = new Subscription();

  constructor(
    private route: ActivatedRoute,
    private title: Title
  ) { }

  ngOnInit(): void {
    this.subscription.add(
      this.route.params.subscribe(params => {
        this.originName = params['origin'];
        this.title.setTitle(`${this.originName} | Habitat Builder`);
      })
    );
  }

  ngOnDestroy(): void {
    this.subscription.unsubscribe();
  }
}
