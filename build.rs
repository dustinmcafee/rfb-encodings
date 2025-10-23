// Copyright 2025 Dustin McAfee
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

fn main() {
    // Only configure linking if turbojpeg feature is enabled
    #[cfg(feature = "turbojpeg")]
    {
        // On macOS, check Homebrew paths
        #[cfg(target_os = "macos")]
        {
            // Try Apple Silicon path first
            if std::path::Path::new("/opt/homebrew/opt/jpeg-turbo/lib").exists() {
                println!("cargo:rustc-link-search=native=/opt/homebrew/opt/jpeg-turbo/lib");
            }
            // Fall back to Intel path
            else if std::path::Path::new("/usr/local/opt/jpeg-turbo/lib").exists() {
                println!("cargo:rustc-link-search=native=/usr/local/opt/jpeg-turbo/lib");
            }
        }

        // On Windows with vcpkg
        #[cfg(target_os = "windows")]
        {
            if let Ok(vcpkg_root) = std::env::var("VCPKG_ROOT") {
                let lib_path = format!("{}/installed/x64-windows/lib", vcpkg_root);
                if std::path::Path::new(&lib_path).exists() {
                    println!("cargo:rustc-link-search=native={}", lib_path);
                }
            }
        }

        // On Linux, the library should be in standard locations
        // so no special configuration needed
    }
}
