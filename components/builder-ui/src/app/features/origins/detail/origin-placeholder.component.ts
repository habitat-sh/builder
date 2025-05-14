import { Component, OnInit, OnDestroy } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ActivatedRoute } from '@angular/router';
import { Subscription } from 'rxjs';

@Component({
  selector: 'app-origin-placeholder',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div class="placeholder-container">
      <h2>{{ title }}</h2>
      <p>This is the {{ title.toLowerCase() }} page for {{ originName }}</p>
      
      <div class="placeholder">
        <p>{{ message }}</p>
      </div>
    </div>
  `,
  styles: [`
    .placeholder-container h2 {
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
export class OriginPlaceholderComponent implements OnInit, OnDestroy {
  originName: string = '';
  title: string = '';
  message: string = '';
  private subscription: Subscription = new Subscription();

  constructor(private route: ActivatedRoute) { }

  ngOnInit(): void {
    // Get the type from route data
    const routeData = this.route.snapshot.data;
    this.title = routeData['title'] || 'Component';
    this.message = routeData['message'] || 'This feature will be implemented soon.';
    
    this.subscription.add(
      this.route.parent?.params.subscribe(params => {
        this.originName = params['origin'];
      })
    );
  }

  ngOnDestroy(): void {
    this.subscription.unsubscribe();
  }
}
