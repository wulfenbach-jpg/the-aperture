import urllib.request
base='https://raw.githubusercontent.com/can1357/oh-my-pi/main/'
for p in ['packages/coding-agent/src/main.ts','packages/coding-agent/src/config/settings.ts','packages/coding-agent/src/cli/args.ts']:
    lines=urllib.request.urlopen(base+p).read().decode().splitlines()
    print('\nFILE',p)
    for i,line in enumerate(lines):
        if any(z in line for z in ['extraConfig','configFiles','configPaths','args.config','deepMerge','fallbackChains','configOverlays','configOverlay']):
            print('\n'.join(lines[max(0,i-3):i+5]))
