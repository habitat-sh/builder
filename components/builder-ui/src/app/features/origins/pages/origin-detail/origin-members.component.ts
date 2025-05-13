import { Component, OnInit, OnDestroy } from '@angular/core';
import { ActivatedRoute } from '@angular/router';
import { Subscription } from 'rxjs';
import { CommonModule } from '@angular/common';
import { MatCardModule } from '@angular/material/card';
import { MatListModule } from '@angular/material/list';

@Component({
  selector: 'app-origin-members',
  templateUrl: './origin-members.component.html',
  styleUrls: ['./origin-members.component.scss'],
  standalone: true,
  imports: [CommonModule, MatCardModule, MatListModule]
})
export class OriginMembersComponent implements OnInit, OnDestroy {
  originName: string = '';
  private subscription: Subscription = new Subscription();

  constructor(private route: ActivatedRoute) { }

  ngOnInit(): void {
    this.subscription.add(
      this.route.parent?.params.subscribe(params => {
        this.originName = params['origin'];
        // In the future, we'll load origin members data here
      })
    );
  }

  ngOnDestroy(): void {
    this.subscription.unsubscribe();
  }
}
