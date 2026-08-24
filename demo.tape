# Demo: verify -> update -> deploy for blog post "How I Joined the Nix Cult"
# Render:  nix develop -c vhs demo.tape
# Outputs: demo.gif (copy to blog repo post dir) + demo.mp4 (LinkedIn/socials)

Output demo.gif

Set FontSize 20
Set Width 1400
Set Height 750
Set Theme "Catppuccin Mocha"
Set TypingSpeed 40ms
Set Shell "fish"
Set Padding 20

Hide
Type "jobsearch update hackernews && jobsearch update upwork && jobsearch update nofluff && jobsearch update efinancialcareers && jobsearch update linkedin && jobsearch update reddit && jobsearch update workatastartup && jobsearch update wellfound && jobsearch embed"
Show
Sleep 2s
Enter
Sleep 200s
