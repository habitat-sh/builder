import { Component, OnInit } from '@angular/core';
import { Title } from '@angular/platform-browser';
import { CommonModule } from '@angular/common';
import { RouterModule } from '@angular/router';
import { MatCardModule } from '@angular/material/card';

@Component({
  selector: 'app-origin-list',
  templateUrl: './origin-list.component.html',
  styleUrls: ['./origin-list.component.scss'],
  standalone: true,
  imports: [CommonModule, RouterModule, MatCardModule]
})
export class OriginListComponent implements OnInit {
  
  constructor(private title: Title) { }

  ngOnInit(): void {
    this.title.setTitle('Origins | Habitat Builder');
  }
}
