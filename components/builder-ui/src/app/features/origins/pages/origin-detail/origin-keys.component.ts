import { Component, OnInit, OnDestroy } from '@angular/core';
import { ActivatedRoute } from '@angular/router';
import { Subscription } from 'rxjs';
import { CommonModule } from '@angular/common';
import { MatCardModule } from '@angular/material/card';

@Component({
  selector: 'app-origin-keys',
  templateUrl: './origin-keys.component.html',
  styleUrls: ['./origin-keys.component.scss'],
  standalone: true,
  imports: [CommonModule, MatCardModule]
})
export class OriginKeysComponent implements OnInit, OnDestroy {
  originName: string = '';
  private subscription: Subscription = new Subscription();

  constructor(private route: ActivatedRoute) { }

  ngOnInit(): void {
    this.subscription.add(
      this.route.parent?.params.subscribe(params => {
        this.originName = params['origin'];
        // In the future, we'll load origin keys data here
      })
    );
  }

  ngOnDestroy(): void {
    this.subscription.unsubscribe();
  }
}
