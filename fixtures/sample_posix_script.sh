#!/bin/sh

ls -l | tr r @ | shasum -a 256
