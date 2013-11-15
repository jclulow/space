#!/usr/bin/env node
/* vim: ts=8 sts=8 sw=8 noet: */

var mod_path = require('path');
var mod_fs = require('fs');

var SCALEMAX; //= 384400000.0;
//var SCALEMAX = 130000000e3;


var NAME = process.argv[2];

if (!NAME) {
	console.error('Usage: %s <universe_name>', './space.js');
	var ents = mod_fs.readdirSync(mod_path.join(__dirname, 'defs')).
	    filter(function (ent) {
		return (ent && ent.match(/\.json$/));
	}).map(function (ent) {
		return (ent.replace(/\.json$/, ''));
	});
	console.error('Found Universes:  ' + ents.join(', '));
	process.exit(1);
}

var OBJECTS = JSON.parse(mod_fs.readFileSync(mod_path.join(__dirname,
    'defs', (process.argv[2] || 'earth') + '.json')));

var SETTINGS = {
	tracelife: 6
};

if (OBJECTS[0].settings) {
	var s = OBJECTS.shift();
	SETTINGS.tracelife = s.tracelife;
}

function
find_scalemax()
{
	var xmax = 0;
	var ymax = 0;

	for (var i = 0; i < OBJECTS.length; i++) {
		var o = OBJECTS[i];

		//console.log('x %d', Math.round(Math.abs(o.x) / 2));
		//console.log('y %d', Math.round(Math.abs(o.y) / 2));
		xmax = Math.max(xmax, Math.round(Math.abs(o.x) / 2));
		ymax = Math.max(ymax, Math.round(Math.abs(o.y) / 2));
	}

	var val = Math.round(Math.max(xmax / 0.6, ymax));

	//console.log('val %d', val);
	//process.exit(0);

	return (val);
}

SCALEMAX = find_scalemax();

var TERM = new (require('./ansiterm').ANSITerm)();

var TRACELIST = [];

var NOW = Date.now();

var XMIN = 2;
var XMAX = process.stdout.columns - 1;
var YMIN = 2;
var YMAX = process.stdout.rows - 1;

function
add_trace(x, y)
{
	for (var i = 0; i < TRACELIST.length; i++) {
		var tl = TRACELIST[i];

		if (tl.x === x && tl.y === y) {
			tl.seen = NOW;
			return;
		}
	}
	TRACELIST.push({
		x: x,
		y: y,
		seen: NOW
	});
}


var GRAV = 6.67e-11;
var XW, XC, YX, YC, RAT, YSP, XSP;

function
space_to_screen_init()
{
	XW = XMAX - XMIN;
	XC = XW / 2 + XMIN;

	YW = YMAX - YMIN;
	YC = YW / 2 + YMIN;

	RAT = YW / XW;

	YSP = SCALEMAX * 1.3;
	XSP = YSP / RAT * 0.444;
}
space_to_screen_init();

function
space_to_screen(x, y)
{
	var x = XC + (XW / 2) * (x / XSP);
	var y = YC + (YW / 2) * (y / YSP);

	return ([x, y].map(Math.round));
}

function
draw_body(o)
{
	var aa = space_to_screen(o.x, o.y);
	if (aa[0] < XMIN || aa[1] < YMIN || aa[0] > XMAX || aa[1] > YMAX)
		return;
	TERM.moveto(aa[0], aa[1]);
	add_trace(aa[0], aa[1]);
	TERM.write('\u001b[38;5;' + (o.colour || '226') + 'm' + o.name + '\u001b[0m');
}

function
draw_space()
{
	TERM.clear();
	TERM.cursor(false);
	TERM.drawBox(XMIN - 1, YMIN - 1, XMAX + 1, YMAX + 1);

	TERM.moveto(Math.floor(XMIN - 1 + (XMAX - XMIN) / 2 - 5), YMIN - 1);
	TERM.write('  S P A C E  ');

	for (var i = 0; i < TRACELIST.length; i++) {
		var tl = TRACELIST[i];
		var intens = Math.floor(255 - (NOW - tl.seen) /
		    (SETTINGS.tracelife * 1000) * (255 - 232));
		if (tl.seen + (SETTINGS.tracelife * 1000) < NOW)
			continue;
		TERM.moveto(tl.x, tl.y);
		space_to_screen_init
		//TERM.write('\u001b[32m' + '·' + '\u001b[0m');
		TERM.write('\u001b[38;5;' + intens + 'm' + '·' + '\u001b[0m');
	}
	for (var i = 0; i < OBJECTS.length; i++) {
		draw_body(OBJECTS[i]);
	}
}

function
move_things()
{
	for (var i = 0; i < OBJECTS.length; i++) {
		var o = OBJECTS[i];

		if (o.fixed)
			continue;

		var Fx = 0;
		var Fy = 0;

		for (var j = 0; j < OBJECTS.length; j++) {
			if (i === j)
				continue;

			var oo = OBJECTS[j];

			var dx = oo.x - o.x;
			var dy = oo.y - o.y;

			var theta = Math.atan2(dy, dx);
			var rsq = dy * dy + dx * dx;

			var Fg = GRAV * o.mass * oo.mass / rsq;

			Fx += Fg * Math.cos(theta);
			Fy += Fg * Math.sin(theta);
		}

		o.vx += Fx / o.mass;
		o.vy += Fy / o.mass;

		o.x += o.vx;
		o.y += o.vy;
	}
}

(function
main()
{
	setInterval(function () {
		NOW = Date.now();
		for (var times = 0; times < 20 * 1000; times++)
			move_things();
		draw_space();
	}, 50);
})();
