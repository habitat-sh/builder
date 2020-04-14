// Copyright (c) 2020 Chef Software Inc. and/or applicable contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::{error, result};

//
// SlowOrdering is, as it is called, slow. It's intended to be the minimal, correct implementation
// There are many ways of doing better. It might still be useful as the ground truth for a quickcheck test
// on a cleverer implementation
#[derive(Copy)]
pub struct SlowOrderingElement {
    key: usize;
}

#[derive(Default)]
pub struct SlowOrdering {
    next_order_key: usize;
    order: Vec<usize>;
}

impl SlowOrdering {
    pub fn new() -> Self {
        SlowOrdering::default()
    }

    pub fn insert_first(mut &self)  -> SlowOrderingElement {
        let new_element = self.new_order_element();
        self.order.insert(0,new_element.key);
        new_element
    }

    pub fn insert_last(mut &self) -> SlowOrderingElement {
        let new_element = self.new_order_element();
        self.order.push(new_element.key);
        new_element
    }

    // This is O(n) the find_element and the shift sum to n steps
    pub fn insert_after(mut &self, element: SlowOrderingElement) -> Result<SlowOrderingElement, ()> {
        if let Some(position) = self.find_element(element) {
            let new_element = self.new_order_element();
            let end = self.order.len()-1;
            self.order.push(self.order[end]); // This could be any value, because it's stepped on
            // start at the end, shift our way back to the insert position
            for i in ((position+1)..end).rev() {
                self.order[i+1] = self.order[i];
            }
            self.order[position+1] = new_element.key;
            Ok(new_element)
        } else 
        {
            Err(())
        }
    }

    fn new_order_element(mut &self) -> SlowOrderingElement {
        let key = next_order_key;
        next_order_key+=1;
        SlowOrderingElement(key)
    }

    fn find_element(&self, element: SlowOrderingElement) -> Option<usize> {
        self.order.iter().position(|i| i == element.key
    }
}