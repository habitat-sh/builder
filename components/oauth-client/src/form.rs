// Copyright (c) 2018 Chef Software Inc. and/or applicable contributors
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

pub(crate) fn encode(fields: &[(&str, &str)]) -> String {
    let mut serializer = url::form_urlencoded::Serializer::new(String::new());

    for (key, value) in fields {
        serializer.append_pair(key, value);
    }

    serializer.finish()
}

#[cfg(test)]
mod test {
    use super::encode;

    #[test]
    fn encode_percent_encodes_values() {
        let encoded = encode(&[("code", "ab+c=="),
                               ("redirect_uri", "https://example.com/cb?x=1")]);

        assert_eq!(encoded,
                   "code=ab%2Bc%3D%3D&redirect_uri=https%3A%2F%2Fexample.com%2Fcb%3Fx%3D1");
    }
}
