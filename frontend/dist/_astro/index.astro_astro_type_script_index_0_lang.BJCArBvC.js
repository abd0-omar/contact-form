async function s(){const n=document.getElementById("xkcd-container"),e=document.getElementById("comic-number");if(!(!n||!e))try{n.innerHTML=`
						<div class="flex justify-center items-center min-h-64">
							<span class="loading loading-spinner loading-lg text-primary"></span>
						</div>
					`,e.textContent="Loading...";const o=await(await fetch("https://corsproxy.io/?https://xkcd.com/info.0.json")).json(),a=Math.floor(Math.random()*o.num)+1,t=await(await fetch(`https://corsproxy.io/?https://xkcd.com/${a}/info.0.json`)).json();e.textContent=`#${t.num}`,n.innerHTML=`
						<h3 class="text-xl font-bold mb-4 text-secondary">
							${t.title}
						</h3>
						
						<div class="mockup-browser bg-base-300 border border-base-300 mb-6">
							<div class="mockup-browser-toolbar">
								<div class="input text-sm">xkcd.com/${t.num}</div>
							</div>
							<div class="bg-base-100 px-6 py-8">
								<img
									src="${t.img}"
									alt="${t.alt}"
									class="mx-auto max-w-full h-auto"
									loading="lazy"
								/>
							</div>
						</div>
						
						<div class="alert alert-info shadow-lg mb-6">
							<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" class="stroke-current shrink-0 w-6 h-6">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
							</svg>
							<div class="text-left">
								<h4 class="font-semibold">Alt Text:</h4>
								<p class="text-sm">${t.alt}</p>
							</div>
						</div>
						
						<div class="card-actions justify-center">
							<div class="join">
								<div class="tooltip" data-tip="Published Date">
									<button class="btn join-item btn-outline btn-sm">
										📅 ${t.day}/${t.month}/${t.year}
									</button>
								</div>
								<button class="btn join-item btn-secondary btn-sm" id="new-random-btn">
									<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" class="w-4 h-4 stroke-current mr-1">
										<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
									</svg>
									New Random
								</button>
								<a
									href="https://xkcd.com/${t.num}"
									target="_blank"
									rel="noopener noreferrer"
									class="btn join-item btn-primary btn-sm"
								>
									<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" class="w-4 h-4 stroke-current">
										<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"></path>
									</svg>
									View on XKCD
								</a>
							</div>
						</div>
					`;const r=document.getElementById("new-random-btn");r&&r.addEventListener("click",s)}catch(i){console.error("Error fetching XKCD comic:",i),n.innerHTML=`
						<div class="alert alert-error">
							<svg xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 w-6 h-6" fill="none" viewBox="0 0 24 24">
								<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z" />
							</svg>
							<div>
								<h3 class="font-bold">Failed to load XKCD comic</h3>
								<p class="text-sm">Please check your internet connection and try again.</p>
							</div>
						</div>
						<div class="mt-4 text-center">
							<button class="btn btn-primary" id="try-again-btn">
								<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" class="w-4 h-4 stroke-current mr-1">
									<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
								</svg>
								Try Again
							</button>
						</div>
					`;const o=document.getElementById("try-again-btn");o&&o.addEventListener("click",s),e.textContent="Error"}}document.addEventListener("DOMContentLoaded",s);
