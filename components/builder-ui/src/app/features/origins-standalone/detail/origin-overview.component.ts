import { Component, OnInit, OnDestroy } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ActivatedRoute } from '@angular/router';
import { Subscription } from 'rxjs';

@Component({
  selector: 'app-origin-overview',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div class="overview-container">
      <h2>Origin Overview</h2>
      <p>This is the overview page for {{ originName }}</p>
      
      <div class="placeholder">
        <p>Origin details and statistics will be displayed here.</p>
      </div>
    </div>
  `,
  styles: [`
    .overview-container h2 {
      margin-top: 0;
    }
    
    .placeholder {
      padding: 16px;
      background-color: #f5f5f5;
      border-radius: 4px;
      margin-top: 16px;
    }
  `]
})
export class OriginOverviewComponent implements OnInit, OnDestroy {
  originName: string = '';
  private subscription: Subscription = new Subscription();

  constructor(private route: ActivatedRoute) { }

  ngOnInit(): void {
    this.subscription.add(
      this.route.parent?.params.subscribe(params => {
        this.originName = params['origin'];
        // In the future, we'll load origin data here
      })
    );
  }

  ngOnDestroy(): void {
    this.subscription.unsubscribe();
  }
}
